//! Reads the option lines of `--help` output and of man pages, such as
//! `-o, --output <FILE>   Write output to FILE`, followed by an indented description when the
//! line has none.

use super::spec::{CommandSpec, Completions, FlagSpec};
use nu_protocol::SyntaxShape;

/// Reads the flags of the command at `path` from its help text or man page.
pub(super) fn parse(text: &str, path: Vec<String>) -> CommandSpec {
    // ANSI stripping also drops backspaces and tabs, so overstrike and tabs go first.
    let text = text
        .lines()
        .map(|line| expand_tabs(&strip_overstrike(line)))
        .collect::<Vec<_>>()
        .join("\n");
    let text = nu_utils::strip_ansi_unlikely(&text);
    let lines = text
        .lines()
        .map(|line| (line.len() - line.trim_start().len(), line.trim()))
        .collect::<Vec<_>>();

    let in_option_section = option_sections(&lines);
    let options = lines
        .iter()
        .zip(in_option_section)
        .map(|((_, line), in_section)| in_section.then(|| parse_option_line(line)).flatten())
        .collect::<Vec<_>>();

    // A description that starts on the line below its option sits deeper than the option. Once
    // that depth is known, a line at that depth that starts with a flag is wrapped prose, such
    // as `-I is specified.`, not an option.
    let description_indent = (0..lines.len().saturating_sub(1))
        .filter(|&index| {
            options[index].is_some()
                && options[index + 1].is_none()
                && lines[index + 1].0 > lines[index].0
        })
        .map(|index| lines[index + 1].0)
        .min();
    let is_option_line = |index: usize| {
        options[index].is_some()
            && description_indent.is_none_or(|description| lines[index].0 < description)
    };

    let mut spec = CommandSpec::new(path);
    for (index, option) in options.iter().enumerate() {
        let Some((spellings, rest)) = option else {
            continue;
        };
        if !is_option_line(index) {
            continue;
        }
        // The description is the rest of the line, then the deeper lines below it, up to a
        // blank line or the next option. Possible values may follow in later paragraphs.
        let mut description = rest.to_string();
        let mut block = vec![*rest];
        let mut in_first_paragraph = true;
        for (offset, &(next_indent, next)) in lines[index + 1..].iter().enumerate() {
            if !next.is_empty()
                && (next_indent <= lines[index].0 || is_option_line(index + 1 + offset))
            {
                break;
            }
            in_first_paragraph &= !next.is_empty();
            if in_first_paragraph {
                description.push(' ');
                description.push_str(next);
            }
            block.push(next);
        }
        let flag = FlagSpec {
            completions: Completions {
                values: possible_values(&block),
                ..Completions::default()
            },
            description: first_sentence(description.trim()).to_string(),
            ..FlagSpec::default()
        };
        add_flags(&mut spec, spellings, flag);
    }
    spec
}

/// The values clap lists for an option, as `[possible values: auto, always, never]` or as a
/// `Possible values:` paragraph of `- auto: ...` lines.
fn possible_values(block: &[&str]) -> Vec<String> {
    let text = block.join(" ");
    if let Some((_, rest)) = text.split_once("[possible values: ")
        && let Some((values, _)) = rest.split_once(']')
    {
        return values
            .split(", ")
            .map(|value| value.trim().to_string())
            .collect();
    }
    block
        .iter()
        .skip_while(|line| !line.eq_ignore_ascii_case("possible values:"))
        .skip(1)
        .map_while(|line| line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")))
        .filter_map(|item| item.split([':', ' ']).next())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

/// Whether each line is in a section that lists options.
///
/// Man pages and some help texts have all-caps section headings (`OPTIONS`, `DESCRIPTION`,
/// `SEARCH OPTIONS:`). Prose in their other sections can start a line with a flag, so options
/// are only read in sections whose heading names options, flags or the description. Text
/// without such headings is read whole.
fn option_sections(lines: &[(usize, &str)]) -> Vec<bool> {
    let is_heading = |&(indent, line): &(usize, &str)| {
        indent == 0
            && !line.starts_with('-')
            && line.chars().any(char::is_alphabetic)
            && !line.chars().any(char::is_lowercase)
    };
    if !lines.iter().any(is_heading) {
        return vec![true; lines.len()];
    }
    let mut in_section = false;
    lines
        .iter()
        .map(|line| {
            if is_heading(line) {
                in_section = ["OPTION", "FLAG", "DESCRIPTION"]
                    .iter()
                    .any(|name| line.1.contains(name));
                false
            } else {
                in_section
            }
        })
        .collect()
}

/// The text up to and including its first `. `, which is enough for a `#` comment.
fn first_sentence(text: &str) -> &str {
    text.find(". ").map_or(text, |end| &text[..=end])
}

/// One flag spelling on an option line, such as `--output <FILE>` or `-u[<mode>]`.
#[derive(Debug, PartialEq)]
struct Spelling {
    /// The name without its dashes.
    name: String,
    long: bool,
    value: Value,
    /// For `--no-name` from `--[no-]name`, the flag it turns off.
    negates: Option<String>,
}

#[derive(Debug, PartialEq)]
enum Value {
    None,
    Required(SyntaxShape),
    Optional,
}

/// Adds the flags of one option line as copies of `flag`, or lists them as skipped when one
/// takes an optional value.
fn add_flags(spec: &mut CommandSpec, spellings: &[Spelling], flag: FlagSpec) {
    if spellings.iter().any(|s| s.value == Value::Optional) {
        for spelling in spellings {
            let dashes = if spelling.long { "--" } else { "-" };
            spec.skip(format!("{dashes}{}", spelling.name));
        }
        return;
    }

    let arg = spellings.iter().find_map(|spelling| match &spelling.value {
        Value::Required(shape) => Some(shape.clone()),
        _ => None,
    });
    let (negated, spellings): (Vec<_>, Vec<_>) =
        spellings.iter().partition(|s| s.negates.is_some());
    let (longs, shorts): (Vec<_>, Vec<_>) = spellings.into_iter().partition(|s| s.long);
    // Values only complete a flag that takes one.
    let completions = match arg {
        Some(_) => flag.completions,
        None => Completions::default(),
    };
    spec.add_names(
        longs.into_iter().map(|s| s.name.clone()).collect(),
        shorts.into_iter().map(|s| s.name.clone()).collect(),
        FlagSpec {
            arg,
            completions,
            ..flag
        },
    );
    // `--no-name` is a switch of its own, described by the flag it turns off.
    for spelling in negated {
        let negates = spelling.negates.as_deref().unwrap_or_default();
        spec.add_names(
            vec![spelling.name.clone()],
            vec![],
            FlagSpec {
                description: format!("Opposite of --{negates}."),
                ..FlagSpec::default()
            },
        );
    }
}

/// Splits an option line into its flag spellings and the description that follows them, or
/// returns `None` when the line doesn't start with a flag.
fn parse_option_line(line: &str) -> Option<(Vec<Spelling>, &str)> {
    let mut spellings = vec![];
    let mut rest = line;
    while rest.starts_with('-') {
        let (spelling, after) = parse_spelling(rest)?;
        spellings.extend(spelling);
        rest = after;

        // Spellings are separated by `, `, ` | ` or a single space.
        let separated = rest.trim_start_matches([',', '|', ' ']);
        if separated.starts_with('-') && !rest.starts_with("  ") {
            rest = separated;
        } else {
            break;
        }
    }
    if spellings.is_empty() {
        return None;
    }

    let rest = rest.trim();
    // A single word after the flags, with nothing after it, names the value (`-D format`).
    if !rest.is_empty()
        && !rest.contains(char::is_whitespace)
        && let Some(last) = spellings.last_mut()
        && last.value == Value::None
    {
        last.value = Value::Required(SyntaxShape::String);
        return Some((spellings, ""));
    }
    Some((spellings, rest))
}

/// Parses one spelling at the start of `text`. `--[no-]name` gives both names.
fn parse_spelling(text: &str) -> Option<(Vec<Spelling>, &str)> {
    let (long, rest) = match text.strip_prefix("--") {
        Some(rest) => (true, rest),
        None => (false, &text[1..]),
    };
    let (negatable, rest) = match rest.strip_prefix("[no-]") {
        Some(rest) => (true, rest),
        None => (false, rest),
    };
    let end = rest
        .find(|c: char| c.is_whitespace() || matches!(c, ',' | '=' | '[' | '<' | '|' | ')'))
        .unwrap_or(rest.len());
    let name = rest[..end].trim_end_matches("...");
    if !name.chars().next().is_some_and(char::is_alphanumeric) {
        return None;
    }
    let (value, rest) = parse_value(&rest[end..]);
    let rest = rest.trim_start_matches("...");

    let mut spellings = vec![Spelling {
        name: name.to_string(),
        long,
        value,
        negates: None,
    }];
    if negatable {
        spellings.push(Spelling {
            name: format!("no-{name}"),
            long,
            value: Value::None,
            negates: Some(name.to_string()),
        });
    }
    Some((spellings, rest))
}

/// Parses the value placeholder right after a flag name: `=SIZE`, `[=WHEN]`, `[<mode>]`,
/// ` <FILE>`, ` FILE` or a Go type name such as ` int`.
fn parse_value(text: &str) -> (Value, &str) {
    if let Some(rest) = text.strip_prefix('[') {
        let end = rest.find(']').map_or(rest.len(), |end| end + 1);
        return (Value::Optional, &rest[end..]);
    }
    if let Some(rest) = text.strip_prefix('=') {
        let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        return (Value::Required(value_shape(&rest[..end])), &rest[end..]);
    }
    if let Some(rest) = text.strip_prefix(" <").or_else(|| text.strip_prefix('<')) {
        let end = rest.find('>').unwrap_or(rest.len());
        let rest_after = rest.get(end + 1..).unwrap_or_default();
        return (Value::Required(value_shape(&rest[..end])), rest_after);
    }
    if let Some(rest) = text.strip_prefix(' ') {
        let end = rest
            .find(|c: char| c.is_whitespace() || c == ',')
            .unwrap_or(rest.len());
        let word = &rest[..end];
        let is_placeholder = !word.is_empty()
            && (word
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || matches!(c, '_' | '-'))
                && word.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                || GO_TYPES.contains(&word));
        if is_placeholder {
            return (Value::Required(value_shape(word)), &rest[end..]);
        }
    }
    (Value::None, text)
}

/// The value types that Go's `flag` and `pflag` packages print after a flag name.
const GO_TYPES: &[&str] = &[
    "string",
    "strings",
    "stringArray",
    "stringToString",
    "int",
    "int32",
    "int64",
    "uint",
    "uint32",
    "uint64",
    "ints",
    "float",
    "float32",
    "float64",
    "duration",
    "bytes",
];

/// The type a value placeholder names. A directory (`<dir>`, `DIRECTORY`) gets the
/// `directory` type, which completes directories.
fn value_shape(placeholder: &str) -> SyntaxShape {
    match placeholder {
        "int" | "int32" | "int64" | "uint" | "uint32" | "uint64" => SyntaxShape::Int,
        "float" | "float32" | "float64" => SyntaxShape::Number,
        _ if ["dir", "directory", "dirname", "folder"]
            .iter()
            .any(|name| placeholder.eq_ignore_ascii_case(name)) =>
        {
            SyntaxShape::Directory
        }
        _ => SyntaxShape::String,
    }
}

/// Removes the overstrike sequences (`X\bX` for bold, `_\bX` for underline) that `man` prints
/// when its output isn't a terminal.
fn strip_overstrike(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if chars.peek() == Some(&'\u{8}') {
            chars.next();
        } else {
            out.push(c);
        }
    }
    out
}

/// Replaces tabs with spaces up to the next tab stop, every 8 columns, so indentation can be
/// compared by counting spaces.
fn expand_tabs(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    for c in line.chars() {
        if c == '\t' {
            let width = 8 - out.chars().count() % 8;
            out.extend(std::iter::repeat_n(' ', width));
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_gnu_and_clap_help() {
        let text = "\
Usage: tool [OPTIONS] <FILE>

Options:
  -a, --all                  do not ignore entries starting with .
      --block-size=SIZE      scale sizes by SIZE
      --color[=WHEN]         color the output WHEN
  -o, --output <FILE>        Write output to FILE
  -v, --verbose...           More output per occurrence
  -h, --help                 Print help
";
        let spec = parse(text, vec![]);
        assert_eq!(
            spec.flag_tuples(),
            [
                (
                    Some("all"),
                    Some('a'),
                    None,
                    "do not ignore entries starting with ."
                ),
                (
                    Some("block-size"),
                    None,
                    Some(SyntaxShape::String),
                    "scale sizes by SIZE"
                ),
                (
                    Some("output"),
                    Some('o'),
                    Some(SyntaxShape::String),
                    "Write output to FILE"
                ),
                (
                    Some("verbose"),
                    Some('v'),
                    None,
                    "More output per occurrence"
                ),
                (Some("help"), Some('h'), None, "Print help"),
            ]
        );
        assert_eq!(spec.skipped, ["--color"]);
    }

    #[test]
    fn reads_clap_possible_values() {
        let text = "\
Options:
      --color <WHEN>  Coloring [default: auto] [possible values: auto, always, never]
  -m, --mode <MODE>
          How to run.

          Possible values:
          - fast: Skip the checks
          - safe: Run every check

  -q, --quiet         Print less
";
        let spec = parse(text, vec![]);
        assert_eq!(
            spec.flags[0].completions.values,
            ["auto", "always", "never"]
        );
        assert_eq!(spec.flags[1].completions.values, ["fast", "safe"]);
        assert_eq!(spec.flags[1].description, "How to run.");
        assert!(spec.flags[2].completions.values.is_empty());
    }

    #[test]
    fn reads_git_and_go_help() {
        let text = "\
    -v, --[no-]verbose    be verbose
    --[no-]porcelain[=<version>]
                          machine-readable output
    -C <path>             run as if git was started in <path>
    --work-tree <dir>     set the path to the working tree
  -n, --namespace string   If present, the namespace scope
      --chunk-size int     Return large lists in chunks
";
        let spec = parse(text, vec![]);
        assert_eq!(
            spec.flag_tuples(),
            [
                (Some("verbose"), Some('v'), None, "be verbose"),
                (Some("no-verbose"), None, None, "Opposite of --verbose."),
                (
                    None,
                    Some('C'),
                    Some(SyntaxShape::String),
                    "run as if git was started in <path>"
                ),
                (
                    Some("work-tree"),
                    None,
                    Some(SyntaxShape::Directory),
                    "set the path to the working tree"
                ),
                (
                    Some("namespace"),
                    Some('n'),
                    Some(SyntaxShape::String),
                    "If present, the namespace scope"
                ),
                (
                    Some("chunk-size"),
                    None,
                    Some(SyntaxShape::Int),
                    "Return large lists in chunks"
                ),
            ]
        );
        assert_eq!(spec.skipped, ["--porcelain", "--no-porcelain"]);
    }

    #[test]
    fn reads_man_pages() {
        // BSD `man ls` as `man` prints it to a pipe, with overstrike for bold and underline.
        let text = "\
     -\u{8}-A\u{8}A      Include directory entries whose names begin with a dot.  Set for
             the super-user unless
             -I is specified.

     -\u{8}-D\u{8}D _\u{8}f_\u{8}o_\u{8}r_\u{8}m_\u{8}a_\u{8}t
             When printing in the long format, use format.

     -s, --short
\t Give the output in the short-format.

     -u[<mode>], --untracked-files[=<mode>]
\t Show untracked files.

     -name pattern
             True if the last component of the pathname matches pattern.
";
        let spec = parse(text, vec![]);
        assert_eq!(
            spec.flag_tuples(),
            [
                (
                    None,
                    Some('A'),
                    None,
                    "Include directory entries whose names begin with a dot."
                ),
                (
                    None,
                    Some('D'),
                    Some(SyntaxShape::String),
                    "When printing in the long format, use format."
                ),
                (
                    Some("short"),
                    Some('s'),
                    None,
                    "Give the output in the short-format."
                ),
            ]
        );
        assert_eq!(spec.skipped, ["-u", "--untracked-files", "-name"]);
    }

    #[test]
    fn reads_only_the_option_sections_of_a_man_page() {
        let text = "\
NAME
     git-status - Show the working tree status

OPTIONS
     -s, --short
         Give the output in the short-format.

BACKGROUND REFRESH
     To avoid this, pass the
     --no-optional-locks option to git.
";
        let spec = parse(text, vec![]);
        assert_eq!(
            spec.flag_tuples(),
            [(
                Some("short"),
                Some('s'),
                None,
                "Give the output in the short-format."
            )]
        );
    }

    #[test]
    fn ignores_lines_that_are_not_options() {
        let text = "\
usage: git status [<options>] [--] [<pathspec>...]
  - a bulleted item
  --------------------
  --  end of options
";
        assert_eq!(parse(text, vec![]), CommandSpec::new(vec![]));
    }
}
