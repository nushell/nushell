//! The commands and flags `generate completions` reads from a source, and how they are written out as
//! `extern` definitions.

use super::completers::{self, Completers, Dynamic};
use nu_protocol::SyntaxShape;
use std::fmt::Write;

/// One command or subcommand of the tool, such as `git` or `git remote add`.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct CommandSpec {
    /// The words of the name after the tool's own, empty for the tool itself.
    pub path: Vec<String>,
    pub description: String,
    pub flags: Vec<FlagSpec>,
    /// Flags the source lists but an `extern` can't declare, as the source spells them.
    pub skipped: Vec<String>,
    /// What the command's arguments, other than flags, complete to.
    pub args: Completions,
}

/// The completions of an argument: fixed values, and completers that run when you press Tab.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Completions {
    pub values: Vec<String>,
    pub dynamic: Vec<Dynamic>,
}

impl Completions {
    /// Adds the values and completers of `other` that this one doesn't have yet.
    pub fn extend(&mut self, other: Completions) {
        for value in other.values {
            if !self.values.contains(&value) {
                self.values.push(value);
            }
        }
        for dynamic in other.dynamic {
            if !self.dynamic.contains(&dynamic) {
                self.dynamic.push(dynamic);
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty() && self.dynamic.is_empty()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct FlagSpec {
    pub long: Option<String>,
    pub short: Option<char>,
    /// The type of the flag's value, or `None` for a switch.
    pub arg: Option<SyntaxShape>,
    /// What the flag's value completes to.
    pub completions: Completions,
    pub description: String,
}

impl CommandSpec {
    pub fn new(path: Vec<String>) -> Self {
        Self {
            path,
            ..Default::default()
        }
    }

    /// Adds a flag, or records it as skipped when an `extern` can't declare its name.
    ///
    /// A flag whose long name or short letter is already declared keeps only the part that is
    /// new, since a signature can't hold the same name twice.
    pub fn add_flag(&mut self, mut flag: FlagSpec) {
        if flag.long.as_deref().is_some_and(|long| !is_flag_name(long))
            || flag
                .short
                .is_some_and(|short| !short.is_ascii_alphanumeric())
        {
            self.skipped.push(flag.spelling());
            return;
        }
        // A name the source also lists with an optional value stays undeclared.
        if flag.long.as_ref().is_some_and(|long| {
            self.skipped.contains(&format!("--{long}"))
                || self.flags.iter().any(|f| f.long.as_ref() == Some(long))
        }) {
            flag.long = None;
        }
        if flag.short.is_some_and(|short| {
            self.skipped.contains(&format!("-{short}"))
                || self.flags.iter().any(|f| f.short == Some(short))
        }) {
            flag.short = None;
        }
        if flag.long.is_some() || flag.short.is_some() {
            self.flags.push(flag);
        }
    }

    /// Adds the flags that one entry of a source declares together, such as `-o, --output`,
    /// each a copy of `flag` with its names.
    ///
    /// The first long and the first short name are one flag, and any further names are flags
    /// of their own. A short name longer than one letter is a single-dash long option such as
    /// `-name`, which is skipped.
    pub fn add_names(&mut self, longs: Vec<String>, shorts: Vec<String>, flag: FlagSpec) {
        let mut longs = longs.into_iter();
        let mut shorts = shorts.into_iter();
        let mut names = vec![(longs.next(), shorts.next())];
        names.extend(longs.map(|long| (Some(long), None)));
        names.extend(shorts.map(|short| (None, Some(short))));

        for (long, short) in names {
            let short = match short {
                Some(short) if short.chars().count() == 1 => short.chars().next(),
                Some(short) => {
                    self.skip(format!("-{short}"));
                    None
                }
                None => None,
            };
            if long.is_some() || short.is_some() {
                self.add_flag(FlagSpec {
                    long,
                    short,
                    ..flag.clone()
                });
            }
        }
    }

    pub fn skip(&mut self, spelling: impl Into<String>) {
        let spelling = spelling.into();
        if !self.skipped.contains(&spelling) {
            self.skipped.push(spelling);
        }
    }

    /// Merges a flag that a later source gives for this command into the one an earlier
    /// source gave for the same long or short name, or adds it.
    ///
    /// The earlier flag keeps its description and names, and gains a name or values it
    /// lacked. When the sources disagree about whether the flag takes a value, it becomes a
    /// switch: a value after a switch still reaches the command, while a typed flag would take
    /// an argument that isn't its value.
    fn merge_flag(&mut self, flag: FlagSpec) {
        // A flag with a long name is the same flag only under that name: `--size(-s)` isn't
        // `--short(-s)`, it just can't have `-s` too.
        let Some(index) = self.flags.iter().position(|known| match &flag.long {
            Some(_) => known.long == flag.long,
            None => flag.short.is_some() && known.short == flag.short,
        }) else {
            self.add_flag(flag);
            return;
        };
        let long_taken = flag.long.is_some() && self.flags.iter().any(|f| f.long == flag.long);
        let short_taken = flag.short.is_some() && self.flags.iter().any(|f| f.short == flag.short);
        let known = &mut self.flags[index];
        if known.long.is_none() && !long_taken {
            known.long = flag.long;
        }
        if known.short.is_none() && !short_taken {
            known.short = flag.short;
        }
        if known.description.is_empty() {
            known.description = flag.description;
        }
        if known.arg.is_some() != flag.arg.is_some() {
            known.arg = None;
            known.completions = Completions::default();
        } else if known.arg.is_some() {
            known.completions.extend(flag.completions);
        }
    }
}

/// Merges the specs of several sources, in order of preference, into one set without
/// duplicates.
///
/// A command is in the result when any source has it. Its description comes from the first
/// source that has one, and its flags are merged by [`CommandSpec::merge_flag`]. A name any
/// source lists as not declarable stays undeclared.
pub(super) fn merge(sources: Vec<Vec<CommandSpec>>) -> Vec<CommandSpec> {
    let mut merged: Vec<CommandSpec> = vec![];
    for spec in sources.into_iter().flatten() {
        // Unlike `spec_at`, no parents are added: learning `git status` gives no `git`.
        let index = match merged.iter().position(|known| known.path == spec.path) {
            Some(index) => index,
            None => {
                merged.push(CommandSpec::new(spec.path.clone()));
                merged.len() - 1
            }
        };
        let known = &mut merged[index];
        if known.description.is_empty() {
            known.description = spec.description;
        }
        for skipped in spec.skipped {
            known.skip(skipped);
        }
        for flag in spec.flags {
            known.merge_flag(flag);
        }
        known.args.extend(spec.args);
    }
    for spec in &mut merged {
        let skipped = std::mem::take(&mut spec.skipped);
        for flag in &mut spec.flags {
            if flag
                .long
                .as_ref()
                .is_some_and(|long| skipped.contains(&format!("--{long}")))
            {
                flag.long = None;
            }
            if flag
                .short
                .is_some_and(|short| skipped.contains(&format!("-{short}")))
            {
                flag.short = None;
            }
        }
        spec.flags
            .retain(|flag| flag.long.is_some() || flag.short.is_some());
        spec.skipped = skipped;
    }
    merged
}

impl FlagSpec {
    /// How the source spells the flag, for the list of skipped flags.
    fn spelling(&self) -> String {
        match (&self.long, self.short) {
            (Some(long), _) => format!("--{long}"),
            (None, Some(short)) => format!("-{short}"),
            (None, None) => String::new(),
        }
    }
}

/// The spec at `path`, added after those of its parents if this is its first statement.
pub(super) fn spec_at<'a>(specs: &'a mut Vec<CommandSpec>, path: &[String]) -> &'a mut CommandSpec {
    let index = match specs.iter().position(|spec| spec.path == path) {
        Some(index) => index,
        None => {
            if let Some((_, parent)) = path.split_last() {
                spec_at(specs, parent);
            }
            specs.push(CommandSpec::new(path.to_vec()));
            specs.len() - 1
        }
    };
    &mut specs[index]
}

/// Whether `name` can be declared as a long flag: the parser turns it into a variable name, so
/// it must be a word made of letters, digits, `-` and `_`.
fn is_flag_name(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphanumeric())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Whether `word` can be part of an extern's name.
pub(super) fn is_command_word(word: &str) -> bool {
    word.chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphanumeric())
        && word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '+' | ':'))
        && word.parse::<f64>().is_err()
}

/// Collapses a description to one line for a `#` comment.
pub(super) fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Writes the specs as a module of `export extern` definitions, the tool itself first, after
/// the completer commands they use.
///
/// For a tool built with cobra (`cobra`), every extern is completed by the tool itself through
/// its `__complete` command, after any fixed values a flag lists. Any other extern gets
/// `@complete external`, so an argument none of its completers covers, such as a branch name,
/// is completed by the external completer the user configured.
///
/// The text works both as a module for `use` and as a script for `source` or an autoload
/// directory. A module can't export an extern with its own name, so for `use` the file must be
/// named something other than `tool`, such as `tool-completions.nu`.
pub(super) fn render(tool: &str, origins: &[String], specs: &[CommandSpec], cobra: bool) -> String {
    let mut out = match origins {
        [origin] => format!("# Generated by `generate completions` from {origin}.\n"),
        _ => {
            let mut header = "# Generated by `generate completions` from:\n".to_string();
            for origin in origins {
                let _ = writeln!(header, "# - {origin}");
            }
            header
        }
    };
    let _ = write!(
        out,
        "# Flags that are not declared here still reach `{tool}`."
    );
    out.push_str(match cobra {
        true => " The tool itself completes\n# the arguments, through its `__complete` command.\n",
        false => " Arguments without a completer\n# here are completed by the external completer in $env.config.completions.external.\n",
    });

    let mut completers = Completers::new(tool);
    let complete = match cobra {
        true => {
            let (name, def) = completers::cobra(tool);
            out.push_str(&def);
            format!("@complete \"{name}\"")
        }
        false => "@complete external".to_string(),
    };

    let mut externs = String::new();
    for spec in specs {
        let words = std::iter::once(tool)
            .chain(spec.path.iter().map(String::as_str))
            .collect::<Vec<_>>();
        let name = words.join(" ");
        externs.push('\n');
        if !spec.description.is_empty() {
            let _ = writeln!(externs, "# {}", one_line(&spec.description));
        }
        // After a blank comment line, the note becomes the extern's extra description.
        if !spec.skipped.is_empty() {
            if !spec.description.is_empty() {
                externs.push_str("#\n");
            }
            let _ = writeln!(
                externs,
                "# Not declared (optional value or unsupported name): {}",
                spec.skipped.join(", ")
            );
        }
        let mut params = vec![];
        for flag in &spec.flags {
            let mut param = match (&flag.long, flag.short) {
                (Some(long), Some(short)) => format!("--{long}(-{short})"),
                (Some(long), None) => format!("--{long}"),
                (None, Some(short)) => format!("-{short}"),
                (None, None) => continue,
            };
            if let Some(arg) = &flag.arg {
                let _ = write!(param, ": {arg}");
                // A cobra tool completes values itself better than a translated completer.
                let dynamic = if cobra {
                    &[][..]
                } else {
                    &flag.completions.dynamic[..]
                };
                let site = format!("{} {}", spec.path.join(" "), param_name(flag));
                if let Some(attach) =
                    completers.attach(site.trim(), &flag.completions.values, dynamic)
                {
                    param.push_str(&attach);
                }
            }
            let description = one_line(&flag.description);
            params.push(match description.is_empty() {
                true => param,
                false => format!("{param:<30} # {description}"),
            });
        }
        // The arguments keep the type an extern's arguments have without a signature.
        if !cobra && !spec.args.is_empty() {
            let site = format!("{} args", spec.path.join(" "));
            if let Some(attach) =
                completers.attach(site.trim(), &spec.args.values, &spec.args.dynamic)
            {
                params.push(format!("...args: external_arg{attach}"));
            }
        }
        let _ = write!(externs, "{complete}\nexport extern \"{name}\" [");
        match params.is_empty() {
            true => externs.push_str("]\n"),
            false => {
                externs.push('\n');
                for param in params {
                    let _ = writeln!(externs, "    {param}");
                }
                externs.push_str("]\n");
            }
        }
    }
    out.push_str(&completers.render());
    out.push_str(&externs);
    out
}

/// A flag's name in the name of its completer: `--color` or `-c`.
fn param_name(flag: &FlagSpec) -> String {
    match (&flag.long, flag.short) {
        (Some(long), _) => format!("--{long}"),
        (None, Some(short)) => format!("-{short}"),
        (None, None) => String::new(),
    }
}

/// A flag as `(long, short, value type, description)`, for comparing in tests.
#[cfg(test)]
pub(super) type FlagTuple<'a> = (Option<&'a str>, Option<char>, Option<SyntaxShape>, &'a str);

#[cfg(test)]
impl CommandSpec {
    pub fn flag_tuples(&self) -> Vec<FlagTuple<'_>> {
        self.flags
            .iter()
            .map(|flag| {
                (
                    flag.long.as_deref(),
                    flag.short,
                    flag.arg.clone(),
                    flag.description.as_str(),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flag(long: Option<&str>, short: Option<char>, arg: Option<SyntaxShape>) -> FlagSpec {
        FlagSpec {
            long: long.map(str::to_string),
            short,
            arg,
            completions: Completions::default(),
            description: "desc".into(),
        }
    }

    #[test]
    fn add_flag_skips_names_an_extern_cannot_declare() {
        let mut spec = CommandSpec::new(vec![]);
        spec.add_flag(flag(Some("a.b"), None, None));
        spec.add_flag(flag(None, Some('@'), None));
        spec.add_flag(flag(Some("dry-run"), Some('n'), None));
        assert_eq!(spec.skipped, ["--a.b", "-@"]);
        assert_eq!(spec.flags, [flag(Some("dry-run"), Some('n'), None)]);
    }

    #[test]
    fn add_flag_keeps_only_new_names() {
        let mut spec = CommandSpec::new(vec![]);
        spec.add_flag(flag(Some("verbose"), Some('v'), None));
        spec.add_flag(flag(Some("version"), Some('v'), None));
        spec.add_flag(flag(Some("verbose"), None, None));
        assert_eq!(
            spec.flags,
            [
                flag(Some("verbose"), Some('v'), None),
                flag(Some("version"), None, None)
            ]
        );
    }

    #[test]
    fn render_writes_one_extern_per_command() {
        let mut root = CommandSpec::new(vec![]);
        root.description = "the tool".into();
        root.add_flag(FlagSpec {
            completions: Completions {
                values: vec!["always".into(), "10".into()],
                ..Completions::default()
            },
            ..flag(Some("color"), Some('c'), Some(SyntaxShape::String))
        });
        let mut sub = CommandSpec::new(vec!["add".into()]);
        sub.skip("--color[=<when>]");
        sub.args.extend(Completions {
            dynamic: vec![Dynamic::Library(completers::Library::Hosts)],
            ..Completions::default()
        });
        let module = render("tool", &["tool --help".into()], &[root, sub], false);
        assert_eq!(
            module.lines().take(4).collect::<Vec<_>>(),
            [
                "# Generated by `generate completions` from tool --help.",
                "# Flags that are not declared here still reach `tool`. Arguments without a completer",
                "# here are completed by the external completer in $env.config.completions.external.",
                "",
            ]
        );
        assert!(module.contains("\ndef \"nu-complete tool hosts\" [] {\n"));
        assert!(module.ends_with(
            "\n\
             # the tool\n\
             @complete external\n\
             export extern \"tool\" [\n    \
                 --color(-c): string@[always '10'] # desc\n\
             ]\n\
             \n\
             # Not declared (optional value or unsupported name): --color[=<when>]\n\
             @complete external\n\
             export extern \"tool add\" [\n    \
                 ...args: external_arg@\"nu-complete tool hosts\"\n\
             ]\n"
        ));
    }

    #[test]
    fn render_lets_a_cobra_tool_complete_itself() {
        let mut root = CommandSpec::new(vec![]);
        root.args.extend(Completions {
            dynamic: vec![Dynamic::Library(completers::Library::Hosts)],
            ..Completions::default()
        });
        let module = render("gh", &["gh --help".into()], &[root], true);
        assert!(module.contains("def \"nu-complete gh\" [place: record] {"));
        assert!(module.contains("^gh __complete ...$args"));
        assert!(module.ends_with("@complete \"nu-complete gh\"\nexport extern \"gh\" []\n"));
        assert!(!module.contains("nu-complete gh hosts"));
    }

    #[test]
    fn merge_keeps_the_first_source_and_fills_gaps() {
        let mut first = CommandSpec::new(vec![]);
        first.description = "from the first source".into();
        first.add_flag(FlagSpec {
            description: String::new(),
            ..flag(Some("verbose"), None, None)
        });
        first.add_flag(FlagSpec {
            completions: Completions {
                values: vec!["always".into()],
                ..Completions::default()
            },
            ..flag(Some("color"), None, Some(SyntaxShape::String))
        });
        first.add_flag(flag(Some("message"), Some('m'), Some(SyntaxShape::String)));

        let mut second = CommandSpec::new(vec![]);
        second.description = "from the second source".into();
        second.add_flag(flag(Some("verbose"), Some('v'), None));
        second.add_flag(FlagSpec {
            completions: Completions {
                values: vec!["always".into(), "never".into()],
                ..Completions::default()
            },
            ..flag(Some("color"), None, Some(SyntaxShape::String))
        });
        // The sources disagree about `--message` taking a value.
        second.add_flag(flag(Some("message"), None, None));
        second.skip("--porcelain");
        second.add_flag(flag(None, Some('x'), None));
        // Another flag with the letter of `--message(-m)`.
        second.add_flag(flag(Some("mode"), Some('m'), None));
        let sub = CommandSpec::new(vec!["add".into()]);

        let mut third = CommandSpec::new(vec![]);
        third.add_flag(flag(Some("porcelain"), None, None));

        let merged = merge(vec![vec![first], vec![second, sub], vec![third]]);
        assert_eq!(merged.len(), 2);
        let root = &merged[0];
        assert_eq!(root.description, "from the first source");
        assert_eq!(
            root.flags,
            [
                flag(Some("verbose"), Some('v'), None),
                FlagSpec {
                    completions: Completions {
                        values: vec!["always".into(), "never".into()],
                        ..Completions::default()
                    },
                    ..flag(Some("color"), None, Some(SyntaxShape::String))
                },
                flag(Some("message"), Some('m'), None),
                flag(None, Some('x'), None),
                flag(Some("mode"), None, None),
            ]
        );
        assert_eq!(root.skipped, ["--porcelain"]);
        assert_eq!(merged[1].path, ["add"]);
    }
}
