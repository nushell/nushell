//! Completions that a source computes when you press Tab, written as nushell completer
//! commands in the generated module.
//!
//! A source describes them as code of its own: fish runs a command substitution such as
//! `(__fish_git_remotes)`, zsh calls a function such as `_hosts`. Two kinds translate:
//! - the shared helpers most tools use, for users, hosts, processes and the like, which map to
//!   a small library of nushell completers ([`Library`]), or to the `directory` type;
//! - fish substitutions that run one command, perhaps through a one-line helper and filtered
//!   with `string replace`, `string match` or `string split` ([`translate_fish`]).
//!
//! Anything else, such as code that reads fish's command line or loops over variables, isn't
//! translated, and the extern's `@complete external` completes that argument instead.

use super::spec::is_command_word;
use std::{collections::HashMap, fmt::Write};

/// A completion that runs when you press Tab.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) enum Dynamic {
    Library(Library),
    /// Nushell code translated from the source, named after what it lists, such as `remotes`.
    /// The code returns records with a `value` and a `description`.
    Code {
        name: String,
        body: String,
    },
}

/// What a shared helper of fish or zsh completes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Shared {
    /// Directories, which the `directory` type completes.
    Directory,
    /// Files, which nushell completes when nothing else does.
    Files,
    Dynamic(Dynamic),
}

/// The completers of the library, which every generated module can use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Library {
    Users,
    Groups,
    Hosts,
    Pids,
    Processes,
    Commands,
    Interfaces,
    MakeTargets,
}

impl Library {
    fn name(self) -> &'static str {
        match self {
            Library::Users => "users",
            Library::Groups => "groups",
            Library::Hosts => "hosts",
            Library::Pids => "pids",
            Library::Processes => "processes",
            Library::Commands => "commands",
            Library::Interfaces => "interfaces",
            Library::MakeTargets => "make targets",
        }
    }

    /// The completer's body, which returns records with a `value`.
    fn body(self) -> &'static str {
        match self {
            Library::Users => "sys users | get name | each { {value: $in} }",
            Library::Groups => "sys users | get groups | flatten | uniq | each { {value: $in} }",
            Library::Hosts => {
                "[($nu.home-dir | path join .ssh config) /etc/ssh/ssh_config /etc/hosts]
        | where ($it | path exists)
        | each {|file|
            open --raw $file | lines | where $it !~ '^\\s*(#|$)' | each {|line|
                if ($file | str ends-with hosts) {
                    $line | split row -r '\\s+' | skip 1
                } else {
                    $line | parse --regex '^\\s*Host\\s+(?<names>.+)$' | get names | split row -r '\\s+'
                }
            }
        }
        | flatten | flatten | where $it !~ '[*?!]' and $it != '' | uniq | each { {value: $in} }"
            }
            Library::Pids => "ps | each {|p| {value: ($p.pid | into string), description: $p.name} }",
            Library::Processes => "ps | get name | uniq | each { {value: $in} }",
            Library::Commands => {
                "$env.PATH | where ($it | path exists) | each {|dir| ls $dir | where type != dir | get name | path basename }
        | flatten | uniq | each { {value: $in} }"
            }
            Library::Interfaces => "sys net | get name | each { {value: $in} }",
            Library::MakeTargets => {
                "[GNUmakefile makefile Makefile] | where ($it | path exists) | first 1 | each {
            open --raw $in | lines | parse --regex '^(?<value>[A-Za-z0-9_][^:=#\\s]*)\\s*:([^=]|$)' | get value
        } | flatten | uniq | each { {value: $in} }"
            }
        }
    }
}

/// What a fish helper, such as `__fish_complete_directories`, completes.
pub(super) fn fish_shared(helper: &str) -> Option<Shared> {
    Some(match helper {
        "__fish_complete_directories" => Shared::Directory,
        "__fish_complete_path" | "__fish_complete_suffix" => Shared::Files,
        "__fish_complete_users" | "__fish_print_users" => {
            Shared::Dynamic(Dynamic::Library(Library::Users))
        }
        "__fish_complete_groups" => Shared::Dynamic(Dynamic::Library(Library::Groups)),
        "__fish_print_hostnames" => Shared::Dynamic(Dynamic::Library(Library::Hosts)),
        "__fish_complete_pids" => Shared::Dynamic(Dynamic::Library(Library::Pids)),
        "__fish_complete_proc" => Shared::Dynamic(Dynamic::Library(Library::Processes)),
        "__fish_complete_command" | "__fish_complete_subcommand" => {
            Shared::Dynamic(Dynamic::Library(Library::Commands))
        }
        "__fish_print_interfaces" => Shared::Dynamic(Dynamic::Library(Library::Interfaces)),
        "__fish_print_make_targets" => Shared::Dynamic(Dynamic::Library(Library::MakeTargets)),
        _ => return None,
    })
}

/// What a zsh completion function, such as `_hosts`, completes.
pub(super) fn zsh_shared(function: &str) -> Option<Shared> {
    Some(match function {
        "_directories" | "_dir_list" => Shared::Directory,
        "_files" | "_path_files" => Shared::Files,
        "_users" => Shared::Dynamic(Dynamic::Library(Library::Users)),
        "_groups" => Shared::Dynamic(Dynamic::Library(Library::Groups)),
        "_hosts" => Shared::Dynamic(Dynamic::Library(Library::Hosts)),
        "_pids" => Shared::Dynamic(Dynamic::Library(Library::Pids)),
        "_command_names" => Shared::Dynamic(Dynamic::Library(Library::Commands)),
        "_net_interfaces" => Shared::Dynamic(Dynamic::Library(Library::Interfaces)),
        _ => return None,
    })
}

/// One word of a fish command line, after quote removal.
pub(super) struct FishWord {
    pub text: String,
    /// Whether the word has unquoted syntax that fish expands: `$`, `(`, `*`, `?`, `{`, a
    /// leading `~`, or a `|` between commands.
    pub expands: bool,
}

/// The fish helper functions a completion script defines, as their statements, and the tool.
pub(super) struct FishScript<'a> {
    pub tool: &'a str,
    pub functions: &'a HashMap<String, Vec<String>>,
    pub words: fn(&str) -> Vec<FishWord>,
}

/// Translates the inside of a fish command substitution, such as `__fish_git_remotes` or
/// `git tag --list | string replace -r '^v' ''`, or returns `None` when it needs fish itself.
pub(super) fn translate_fish(substitution: &str, script: &FishScript) -> Option<Shared> {
    let words = (script.words)(substitution);
    if let [helper, rest @ ..] = &words[..]
        && let Some(shared) = fish_shared(&helper.text)
    {
        // `__fish_complete_suffix .tar` still completes files.
        return (rest.iter().all(|word| !word.expands) || shared == Shared::Files)
            .then_some(shared);
    }
    let name = name_of(&words, script)?;
    let pipelines = fish_pipelines(&words, script, 0)?;
    let body = match &pipelines[..] {
        [pipeline] => format!("try {{ {pipeline} }} catch {{ [] }}"),
        _ => format!(
            "[\n        {}\n    ] | flatten | uniq-by value",
            pipelines
                .iter()
                .map(|pipeline| format!("(try {{ {pipeline} }} catch {{ [] }})"))
                .collect::<Vec<_>>()
                .join("\n        ")
        ),
    };
    Some(Shared::Dynamic(Dynamic::Code { name, body }))
}

/// A name for the completer: the helper's name without its `__fish_<tool>_` prefix
/// (`__fish_git_remotes` is `remotes`), or, for the tool itself, its first argument
/// (`__fish_git config -l` is `config`).
fn name_of(words: &[FishWord], script: &FishScript) -> Option<String> {
    let (head, args) = words.split_first()?;
    let runs_tool = head.text == format!("__fish_{}", script.tool)
        || head.text == script.tool
        || head.text == "command";
    let name = match runs_tool {
        true => args
            .iter()
            .map(|arg| arg.text.as_str())
            .find(|arg| *arg != script.tool && !arg.starts_with('-'))
            .unwrap_or("values")
            .to_string(),
        false => head
            .text
            .strip_prefix("__fish_")
            .map(|rest| rest.strip_prefix(script.tool).unwrap_or(rest))
            .unwrap_or(&head.text)
            .to_string(),
    };
    let name = name.trim_matches(['_', '-']).replace('_', " ");
    // The name is part of a command's name, so it must be made of command words.
    let valid = !name.is_empty() && name.split(' ').all(is_command_word);
    Some(if valid { name } else { "values".into() })
}

/// The nushell pipelines a fish command line runs, each returning completion records: one for
/// a single command, or one per statement of a helper function.
fn fish_pipelines(words: &[FishWord], script: &FishScript, depth: usize) -> Option<Vec<String>> {
    let mut commands = words.split(|word| word.expands && word.text == "|");
    let first = commands.next()?;
    let filters = commands.collect::<Vec<_>>();
    let (head, args) = first.split_first()?;

    // A helper function of the script runs its statements, if it takes no arguments here.
    if let Some(body) = script.functions.get(&head.text)
        && head.text != format!("__fish_{}", script.tool)
    {
        if !args.is_empty() || !filters.is_empty() || depth > 3 {
            return None;
        }
        let mut pipelines = vec![];
        for statement in body {
            let words = (script.words)(statement);
            if words.is_empty() {
                continue;
            }
            pipelines.extend(fish_pipelines(&words, script, depth + 1)?);
        }
        return (!pipelines.is_empty()).then_some(pipelines);
    }

    // `__fish_git` and `command git` run git.
    let (program, args) = match head.text.as_str() {
        "command" => args.split_first()?,
        _ => (head, args),
    };
    let program = match program.text.strip_prefix("__fish_") {
        Some(tool) if tool == script.tool => script.tool.to_string(),
        Some(_) => return None,
        None if FISH_BUILTINS.contains(&program.text.as_str()) || program.expands => return None,
        None => program.text.clone(),
    };

    // External filters such as `| grep x` run on the command's output; once a `string` filter
    // runs, the output is nushell's lines.
    let mut code = format!("^{program}{}", external_args(args)?);
    let mut lines = false;
    for filter in filters {
        let (head, args) = filter.split_first()?;
        if head.text == "string" {
            if !lines {
                code.push_str(LINES);
                lines = true;
            }
            code.push_str(&string_filter(args)?);
        } else if !lines && !head.expands && !FISH_BUILTINS.contains(&head.text.as_str()) {
            let _ = write!(code, " | ^{}{}", head.text, external_args(args)?);
        } else {
            return None;
        }
    }
    if !lines {
        code.push_str(LINES);
    }
    Some(vec![format!(
        "{code} | nu-complete {} records",
        script.tool
    )])
}

/// The stdout of the external commands as lines; their errors and exit code are ignored.
const LINES: &str = " | complete | get stdout | lines";

/// Commands that fish runs itself, which have no external program. The keywords are among
/// them, so a helper with `if` or `for` isn't translated.
const FISH_BUILTINS: &[&str] = &[
    "if",
    "else",
    "end",
    "for",
    "while",
    "switch",
    "case",
    "begin",
    "and",
    "or",
    "not",
    "return",
    "break",
    "continue",
    "function",
    "string",
    "echo",
    "printf",
    "set",
    "test",
    "math",
    "path",
    "contains",
    "count",
    "commandline",
    "argparse",
    "status",
    "functions",
    "type",
    "read",
    "builtin",
    "eval",
    "source",
    "set_color",
];

/// The arguments of an external command as nushell words, or `None` when one expands in fish.
/// Redirections such as `2>/dev/null` are dropped: the completer reads only stdout.
fn external_args(args: &[FishWord]) -> Option<String> {
    let mut out = String::new();
    for arg in args {
        if is_redirection(&arg.text) {
            continue;
        }
        if arg.expands {
            return None;
        }
        out.push(' ');
        // An external command's flags, such as `--get-regexp`, need no quotes.
        let name = arg.text.trim_start_matches('-');
        match !name.is_empty() && name.len() < arg.text.len() && nu_word(name) == name {
            true => out.push_str(&arg.text),
            false => out.push_str(&nu_word(&arg.text)),
        }
    }
    Some(out)
}

fn is_redirection(word: &str) -> bool {
    let word = word.trim_start_matches(|c: char| c.is_ascii_digit() || c == '&');
    word.starts_with('>') || word.starts_with('<')
}

/// A fish `string` command as a nushell filter on the lines, or `None` for one that doesn't
/// translate.
fn string_filter(args: &[FishWord]) -> Option<String> {
    if args.iter().any(|arg| arg.expands) {
        return None;
    }
    let (subcommand, args) = args.split_first()?;
    let (options, operands): (Vec<_>, Vec<_>) = args
        .iter()
        .map(|arg| arg.text.as_str())
        .filter(|arg| *arg != "--")
        .partition(|arg| arg.starts_with('-') && arg.len() > 1);
    let has = |flag: char| {
        options.iter().any(|option| {
            !option.starts_with("--") && option[1..].contains(flag)
                || option
                    .strip_prefix("--")
                    .is_some_and(|long| long.starts_with(flag))
        })
    };
    // `-i` ignores case, `-q` prints nothing, `-n` prints positions and `-m` limits matches.
    if has('i') || has('q') || has('n') || has('m') {
        return None;
    }
    match (subcommand.text.as_str(), &operands[..]) {
        ("replace", [pattern, replacement]) => {
            let regex = has('r');
            let mut filter = String::new();
            if has('f') {
                let _ = match regex {
                    true => write!(filter, " | where $it =~ {}", nu_word(pattern)),
                    false => write!(filter, " | where ($it | str contains {})", nu_word(pattern)),
                };
            }
            let all = if has('a') { " -a" } else { "" };
            let regex_flag = if regex { " -r" } else { "" };
            let _ = write!(
                filter,
                " | str replace{all}{regex_flag} {} {}",
                nu_word(pattern),
                nu_replacement(replacement)
            );
            Some(filter)
        }
        ("match", [pattern]) => {
            let regex = has('r');
            // `string match -r` without `-v` prints the matching part, not the line.
            if regex && !has('v') {
                return None;
            }
            let pattern = if regex {
                pattern.to_string()
            } else {
                glob_to_regex(pattern)
            };
            let operator = if has('v') { "!~" } else { "=~" };
            Some(format!(" | where $it {operator} {}", nu_word(&pattern)))
        }
        ("split", [separator]) if options.is_empty() => Some(format!(
            " | each {{ split row {} }} | flatten",
            nu_word(separator)
        )),
        ("trim", []) => Some(" | str trim".to_string()),
        _ => None,
    }
}

/// A fish glob pattern, as used by `string match`, as an anchored regex.
fn glob_to_regex(glob: &str) -> String {
    let mut regex = String::from("^");
    for c in glob.chars() {
        match c {
            '*' => regex.push_str(".*"),
            '?' => regex.push('.'),
            _ if "\\.+()[]{}|^$".contains(c) => {
                regex.push('\\');
                regex.push(c);
            }
            _ => regex.push(c),
        }
    }
    regex.push('$');
    regex
}

/// A word for nushell: as it is when it can't be read as anything but a string, quoted
/// otherwise. A word that starts with a digit can be a number, filesize, duration, date or
/// range, and a comma separates the items of a list.
pub(super) fn nu_word(text: &str) -> String {
    let plain = text
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '/')
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./:=@+".contains(c))
        && text.parse::<f64>().is_err()
        && !matches!(text, "true" | "false" | "null");
    if plain {
        text.to_string()
    } else if text.contains(char::is_control) {
        // Tabs and newlines are written as escapes.
        let mut out = String::from("\"");
        for c in text.chars() {
            match c {
                '\t' => out.push_str("\\t"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '"' | '\\' => {
                    out.push('\\');
                    out.push(c);
                }
                _ => out.push(c),
            }
        }
        out.push('"');
        out
    } else if !text.contains('\'') {
        format!("'{text}'")
    } else {
        format!("r#'{text}'#")
    }
}

/// A replacement of `string replace`, in which `\t` and `\n` are a tab and a newline, as a
/// nushell string.
fn nu_replacement(text: &str) -> String {
    let mut out = String::from("\"");
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some('t') => out.push_str("\\t"),
                Some('n') => out.push_str("\\n"),
                Some(other) => {
                    out.push_str("\\\\");
                    if other == '"' {
                        out.push('\\');
                    }
                    out.push(other);
                }
                None => out.push_str("\\\\"),
            },
            '"' => out.push_str("\\\""),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The completer commands a module defines, and the names its parameters use for them.
pub(super) struct Completers<'a> {
    tool: &'a str,
    /// The definitions as name, input and output types, and body, in the order they were
    /// first used.
    defs: Vec<(String, &'static str, String)>,
    /// The name of each body, so the same completer is defined once.
    names: HashMap<String, String>,
}

impl<'a> Completers<'a> {
    pub fn new(tool: &'a str) -> Self {
        Self {
            tool,
            defs: vec![],
            names: HashMap::new(),
        }
    }

    /// The completion to attach to a parameter with these values and dynamic completions, such
    /// as `@[always never]` or `@"nu-complete git remotes"`, or `None` when it has neither.
    /// `site` names a completer that combines several, such as `checkout args`.
    pub fn attach(&mut self, site: &str, values: &[String], dynamic: &[Dynamic]) -> Option<String> {
        match (values, dynamic) {
            ([], []) => None,
            (values, []) => Some(format!(
                "@[{}]",
                values
                    .iter()
                    .map(|value| nu_word(value))
                    .collect::<Vec<_>>()
                    .join(" ")
            )),
            ([], [one]) => Some(format!("@\"{}\"", self.define(one))),
            (values, dynamic) => {
                let mut parts = dynamic
                    .iter()
                    .map(|dynamic| format!("...({})", self.define(dynamic)))
                    .collect::<Vec<_>>();
                if !values.is_empty() {
                    parts.push(format!(
                        "...([{}] | each {{ {{value: $in}} }})",
                        values
                            .iter()
                            .map(|value| nu_word(value))
                            .collect::<Vec<_>>()
                            .join(" ")
                    ));
                }
                // A value two sources list, such as a branch that is local and remote, once.
                let name = self.add(site, format!("[{}] | uniq-by value", parts.join(", ")));
                Some(format!("@\"{name}\""))
            }
        }
    }

    /// Defines the completer for `dynamic`, if it isn't yet, and returns its name.
    fn define(&mut self, dynamic: &Dynamic) -> String {
        match dynamic {
            Dynamic::Library(library) => self.add(library.name(), library.body().to_string()),
            Dynamic::Code { name, body } => {
                self.add_records_helper();
                self.add(name, body.clone())
            }
        }
    }

    /// The helper that turns `value<TAB>description` lines into completion records.
    fn add_records_helper(&mut self) {
        self.add_typed(
            "records",
            ": list<string> -> list<record>",
            "each {|line| let parts = $line | split row \"\\t\"; {value: $parts.0, description: ($parts.1? | default \"\" | str trim)} }\n    | where value != \"\""
                .to_string(),
        );
    }

    fn add(&mut self, name: &str, body: String) -> String {
        self.add_typed(name, "", body)
    }

    /// Adds a definition named `nu-complete <tool> <name>`, or another name when that one is
    /// taken by a different body, and returns its full name.
    fn add_typed(&mut self, name: &str, types: &'static str, body: String) -> String {
        if let Some(name) = self.names.get(&body) {
            return name.clone();
        }
        let base = format!("nu-complete {} {name}", self.tool);
        let mut full = base.clone();
        let mut n = 2;
        while self.defs.iter().any(|(taken, ..)| *taken == full) {
            full = format!("{base} {n}");
            n += 1;
        }
        self.names.insert(body.clone(), full.clone());
        self.defs.push((full.clone(), types, body));
        full
    }

    /// The definitions, in the order they were first used.
    pub fn render(&self) -> String {
        let mut out = String::new();
        for (name, types, body) in &self.defs {
            let _ = write!(out, "\ndef \"{name}\" []{types} {{\n    {body}\n}}\n");
        }
        out
    }
}

/// The completer for a tool built with cobra, which completes its own command lines through
/// its hidden `__complete` command, attached to every extern of the tool.
pub(super) fn cobra(tool: &str) -> (String, String) {
    let name = format!("nu-complete {tool}");
    let def = format!(
        "\n# Asks `{tool}` itself, through the `__complete` command of cobra.\n\
         def \"{name}\" [place: record] {{\n    \
             # The first word is the extern's whole name, such as `gh issue list`.\n    \
             let args = ($place.command | first | split row ' ' | skip 1) ++ ($place.command | skip 1)\n    \
             let lines = try {{ ^{tool} __complete ...$args | complete | get stdout | lines }} catch {{ [] }}\n    \
             let completions = $lines | where $it !~ '^:' | each {{|line|\n        \
                 let parts = $line | split row \"\\t\"\n        \
                 {{value: $parts.0, description: ($parts.1? | default \"\")}}\n    \
             }}\n    \
             # Nothing to offer: let nushell complete files.\n    \
             if ($completions | is-empty) {{ null }} else {{ $completions }}\n\
         }}\n"
    );
    (name, def)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_words_nushell_reads_as_other_types() {
        let words = [
            "always",
            "refs/heads/",
            "1kb",
            "0x10",
            "5sec",
            "2024-01-01",
            "a,b",
            "true",
        ]
        .map(nu_word);
        assert_eq!(
            words,
            [
                "always",
                "refs/heads/",
                "'1kb'",
                "'0x10'",
                "'5sec'",
                "'2024-01-01'",
                "'a,b'",
                "'true'"
            ]
        );
    }
}
