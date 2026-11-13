//! Reads a zsh completion function: the option specs it passes to `_arguments`, such as
//! `'(-s --short)'{-s,--short}'[output in short format]'`.
//!
//! Which subcommand a spec belongs to comes from where it is written. zsh's own completion
//! functions give each subcommand a function named `_<tool>-<subcommand>`, and the ones clap
//! generates dispatch on the subcommand word with `case $line[1] in (build) ...`. Specs in any
//! other function are skipped, since helper functions may be shared by several subcommands.
//! Subcommand descriptions come from `name:description` entries of arrays whose name mentions
//! commands, such as `commands=('build:Compile the package')`.

#[cfg(test)]
use super::completers::Dynamic;
use super::completers::{self, Shared};
use super::spec::{CommandSpec, Completions, FlagSpec, is_command_word, spec_at};
use nu_protocol::SyntaxShape;

/// Reads the specs of `tool` from its zsh completion function, starting at the subcommand
/// `path` (empty for the tool itself).
pub(super) fn parse(script: &str, tool: &str, path: &[String]) -> Vec<CommandSpec> {
    let mut walk = Walk {
        tool,
        specs: vec![CommandSpec::new(vec![])],
        ..Walk::default()
    };
    for token in tokens(script) {
        walk.token(token);
    }

    let mut specs = walk.specs;
    for group in pair_short_and_long(walk.groups) {
        for path in &group.paths {
            let spec = spec_at(&mut specs, path);
            if group.value == Value::Optional {
                for (name, long) in &group.names {
                    spec.skip(format!("{}{name}", if *long { "--" } else { "-" }));
                }
                continue;
            }
            let (longs, shorts): (Vec<_>, Vec<_>) = group.names.iter().partition(|(_, long)| *long);
            spec.add_names(
                longs.into_iter().map(|(name, _)| name.clone()).collect(),
                shorts.into_iter().map(|(name, _)| name.clone()).collect(),
                FlagSpec {
                    arg: (group.value == Value::Required).then_some(match group.directory {
                        true => SyntaxShape::Directory,
                        false => SyntaxShape::String,
                    }),
                    completions: group.completions.clone(),
                    description: group.description.clone(),
                    ..FlagSpec::default()
                },
            );
        }
    }
    specs.retain(|spec| spec.path.starts_with(path));
    specs
}

/// The option specs of one word of the script. A word like `{-s,--short}'[short format]'`
/// expands to several specs, which name one flag.
#[derive(Debug)]
struct Group {
    paths: Vec<Vec<String>>,
    /// The names without their dashes, and whether each is a long name.
    names: Vec<(String, bool)>,
    value: Value,
    completions: Completions,
    /// Whether the value is a directory.
    directory: bool,
    description: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Value {
    None,
    /// Given in the same word or the next one: `-o+`, `--long=`, or no mark.
    Required,
    /// Optional (`::message`), or only allowed in the same word (`--long=-`, `-o-`). A
    /// signature can't declare either: a typed flag takes the next word as its value.
    Optional,
}

/// One option spec of `_arguments`.
struct Spec {
    /// The name without its dashes.
    name: String,
    long: bool,
    value: Value,
    /// What its action completes, such as the values of `(always never auto)`.
    completions: Completions,
    /// Whether its action completes directories.
    directory: bool,
    description: String,
}

/// clap writes the short and the long name of a flag as two specs, one after the other, with
/// the same description. Those become one group.
fn pair_short_and_long(groups: Vec<Group>) -> Vec<Group> {
    let mut paired: Vec<Group> = vec![];
    for group in groups {
        if let Some(last) = paired.last_mut()
            && let ([(_, last_long)], [(_, long)]) = (&last.names[..], &group.names[..])
            && last_long != long
            && last.paths == group.paths
            && last.value == group.value
            && last.description == group.description
        {
            last.names.extend(group.names);
            last.completions.extend(group.completions);
            last.directory |= group.directory;
            continue;
        }
        paired.push(group);
    }
    paired
}

/// Where the walk is in the script.
#[derive(Default)]
struct Walk<'a> {
    tool: &'a str,
    specs: Vec<CommandSpec>,
    groups: Vec<Group>,
    braces: usize,
    /// The function bodies the walk is in: the subcommand path each one is for (`None` for a
    /// helper function), and the brace depth of its body.
    functions: Vec<(Option<Vec<String>>, usize)>,
    /// A function header was read, and its body starts at the next `{`.
    pending_function: Option<Option<Vec<String>>>,
    after_function_keyword: bool,
    previous_word: String,
    cases: Vec<Case>,
    /// Inside the array assignment `name=( ... )`, whether `name` mentions commands.
    array: Option<bool>,
}

struct Case {
    /// Whether the case dispatches on a command-line word, `case $line[1] in`.
    on_word: bool,
    state: CaseState,
}

enum CaseState {
    Subject,
    BeforeIn,
    Pattern(String),
    /// In an arm, with the subcommand names its pattern lists, or `None` for a pattern such as
    /// `(*)` that names no subcommand.
    Arm(Option<Vec<String>>),
}

impl Walk<'_> {
    fn token(&mut self, token: Token) {
        match token {
            Token::Separator => {}
            // `|` separates the alternatives of a case pattern, as in `add|rm)`.
            Token::Pipe => {
                if let Some(Case {
                    state: CaseState::Pattern(pattern),
                    ..
                }) = self.cases.last_mut()
                    && !pattern.is_empty()
                {
                    pattern.push('|');
                }
            }
            Token::ArmEnd => {
                if let Some(case) = self.cases.last_mut() {
                    case.state = CaseState::Pattern(String::new());
                }
            }
            Token::Word {
                expansions,
                opens_array,
                closes,
            } => {
                self.word(&expansions, opens_array, closes);
                if let Some(first) = expansions.into_iter().next() {
                    self.previous_word = first;
                }
            }
        }
    }

    /// Reads one word, which may start an array (`opens_array`) or end an array or a case
    /// pattern (`closes`).
    fn word(&mut self, expansions: &[String], opens_array: bool, closes: bool) {
        let Some(word) = expansions.first() else {
            return;
        };

        if let Some(case) = self.cases.last_mut() {
            match &mut case.state {
                CaseState::Subject => {
                    case.on_word = is_word_subject(word);
                    case.state = CaseState::BeforeIn;
                    return;
                }
                CaseState::BeforeIn => {
                    if word == "in" {
                        case.state = CaseState::Pattern(String::new());
                    }
                    return;
                }
                CaseState::Pattern(pattern) if word != "esac" => {
                    pattern.push_str(word);
                    if closes {
                        let names = pattern_names(pattern);
                        case.state = CaseState::Arm(names);
                        if case.on_word {
                            for path in self.context().unwrap_or_default() {
                                spec_at(&mut self.specs, &path);
                            }
                        }
                    }
                    return;
                }
                _ => {}
            }
        }

        if self.after_function_keyword {
            self.after_function_keyword = false;
            self.pending_function = Some(self.function_path(word));
            return;
        }
        // A function header is followed by its body, `{`.
        let pending_function = self.pending_function.take();

        match word.as_str() {
            "case" => {
                self.cases.push(Case {
                    on_word: false,
                    state: CaseState::Subject,
                });
                return;
            }
            "esac" => {
                self.cases.pop();
                return;
            }
            "function" => {
                self.after_function_keyword = true;
                return;
            }
            "{" => {
                self.braces += 1;
                if let Some(path) = pending_function {
                    if let Some(path) = path.as_ref().filter(|path| !path.is_empty()) {
                        spec_at(&mut self.specs, path);
                    }
                    self.functions.push((path, self.braces));
                }
                return;
            }
            "}" => {
                if self
                    .functions
                    .last()
                    .is_some_and(|(_, depth)| *depth == self.braces)
                {
                    self.functions.pop();
                }
                self.braces = self.braces.saturating_sub(1);
                return;
            }
            "()" => {
                let name = self.previous_word.clone();
                self.pending_function = Some(self.function_path(&name));
                return;
            }
            _ => {}
        }
        if let Some(name) = word.strip_suffix("()") {
            self.pending_function = Some(self.function_path(name));
            return;
        }

        // `name=( items... )`: the first item may follow `=(` in the same word.
        let mut expansions = expansions.to_vec();
        if opens_array && let Some((name, _)) = word.split_once("=(") {
            self.array = Some(name.contains("command") || name.contains("cmds"));
            for expansion in &mut expansions {
                if let Some((_, first_item)) = expansion.split_once("=(") {
                    *expansion = first_item.to_string();
                }
            }
        }
        let closes_array = self.array.is_some() && closes;
        if closes_array {
            for expansion in &mut expansions {
                expansion.pop();
            }
        }

        if let Some(paths) = self.context() {
            if self.array == Some(true) {
                self.add_subcommands(&expansions, &paths);
            } else {
                self.add_group(&expansions, paths);
            }
        }
        if closes_array {
            self.array = None;
        }
    }

    /// Adds the subcommands that `name:description` items of a commands array list.
    fn add_subcommands(&mut self, items: &[String], paths: &[Vec<String>]) {
        for item in items {
            let Some((name, description)) = item.split_once(':') else {
                continue;
            };
            if !is_command_word(name) {
                continue;
            }
            for path in paths {
                let mut sub_path = path.clone();
                sub_path.push(name.to_string());
                let spec = spec_at(&mut self.specs, &sub_path);
                if spec.description.is_empty() {
                    spec.description = unescape(description);
                }
            }
        }
    }

    /// Adds the option specs that one word expands to, which name one flag.
    fn add_group(&mut self, expansions: &[String], paths: Vec<Vec<String>>) {
        // A positional spec completes the command's arguments.
        for completions in expansions
            .iter()
            .filter_map(|expansion| parse_positional(expansion))
        {
            for path in &paths {
                spec_at(&mut self.specs, path)
                    .args
                    .extend(completions.clone());
            }
        }
        let specs = expansions
            .iter()
            .filter_map(|expansion| parse_spec(expansion))
            .collect::<Vec<_>>();
        if specs.is_empty() {
            return;
        }
        let value = [Value::Optional, Value::Required]
            .into_iter()
            .find(|value| specs.iter().any(|spec| spec.value == *value))
            .unwrap_or(Value::None);
        let mut completions = Completions::default();
        for spec in &specs {
            completions.extend(spec.completions.clone());
        }
        self.groups.push(Group {
            paths,
            completions,
            directory: specs.iter().any(|spec| spec.directory),
            description: specs
                .iter()
                .map(|spec| spec.description.clone())
                .find(|description| !description.is_empty())
                .unwrap_or_default(),
            names: specs
                .into_iter()
                .map(|spec| (spec.name, spec.long))
                .collect(),
            value,
        });
    }

    /// The subcommand paths of the current position, or `None` in a helper function or a
    /// `case` arm that names no subcommand.
    fn context(&self) -> Option<Vec<Vec<String>>> {
        let mut paths = match self.functions.last() {
            Some((path, _)) => vec![path.clone()?],
            None => vec![vec![]],
        };
        for case in self.cases.iter().filter(|case| case.on_word) {
            match &case.state {
                CaseState::Arm(Some(names)) => {
                    paths = paths
                        .iter()
                        .flat_map(|path| {
                            names.iter().map(move |name| {
                                let mut path = path.clone();
                                path.push(name.clone());
                                path
                            })
                        })
                        .collect();
                }
                CaseState::Arm(None) => return None,
                _ => {}
            }
        }
        Some(paths)
    }

    /// The subcommand path a function completes: `_git` (or `__git`) the tool, `_git-status`
    /// a subcommand, and the `_tool_commands` and `_tool__sub_commands` functions of clap list
    /// the subcommands of the tool and of `sub`. Any other function is a helper, including
    /// `_make-expandVars`: subcommand names are lowercase.
    fn function_path(&self, name: &str) -> Option<Vec<String>> {
        let main = format!("_{}", self.tool);
        if name == main || name == format!("_{main}") || name == format!("{main}_commands") {
            return Some(vec![]);
        }
        if let Some(sub) = name.strip_prefix(&format!("{main}-")) {
            return (is_command_word(sub) && !sub.contains(|c: char| c.is_ascii_uppercase()))
                .then(|| vec![sub.to_string()]);
        }
        let path = name
            .strip_prefix(&format!("{main}__"))?
            .strip_suffix("_commands")?
            .split("__")
            .map(str::to_string)
            .collect::<Vec<_>>();
        path.iter()
            .all(|word| is_command_word(word))
            .then_some(path)
    }
}

/// Whether a `case` subject is a command-line word such as `$line[1]` or `${words[2]}`.
fn is_word_subject(subject: &str) -> bool {
    ["line[", "words["].iter().any(|array| {
        subject.split(array).nth(1).is_some_and(|rest| {
            rest.split(']')
                .next()
                .is_some_and(|index| !index.is_empty() && index.chars().all(|c| c.is_ascii_digit()))
        })
    })
}

/// The subcommand names of a case pattern such as `(build|b)`, or `None` when an alternative
/// isn't a plain word.
fn pattern_names(pattern: &str) -> Option<Vec<String>> {
    let inner = pattern.trim_start_matches('(').trim_end_matches(')');
    let names = inner.split('|').map(str::to_string).collect::<Vec<_>>();
    names
        .iter()
        .all(|name| is_command_word(name))
        .then_some(names)
}

/// Parses an option spec of `_arguments`: optional `*` and `!` marks and an exclusion list
/// `(...)`, the name with how its value is attached (`-o+`, `--long=`, `--long=-`, `-o-`), a
/// `[description]`, then `:message:action` for a value or `::message:action` for an optional
/// one.
///
/// A word without a description or a value, such as `_arguments`'s own `-s`, isn't read as a
/// spec, since it is just as likely to be another command's option.
fn parse_spec(text: &str) -> Option<Spec> {
    let mut rest = text;
    loop {
        if let Some(after) = rest.strip_prefix(['*', '!']) {
            rest = after;
        } else if rest.starts_with('(') {
            rest = &rest[rest.find(')')? + 1..];
        } else {
            break;
        }
    }
    if !rest.starts_with('-') {
        return None;
    }

    let end = rest.find(['[', ':']).unwrap_or(rest.len());
    let (mut name, tail) = rest.split_at(end);
    if tail.is_empty() {
        return None;
    }
    let mut attached_only = false;
    for suffix in ["=-", "=", "+", "-"] {
        if let Some(stripped) = name.strip_suffix(suffix)
            && stripped.ends_with(|c: char| c.is_ascii_alphanumeric())
        {
            name = stripped;
            attached_only = suffix.ends_with('-');
            break;
        }
    }
    let (long, name) = match name.strip_prefix("--") {
        Some(name) => (true, name),
        None => (false, &name[1..]),
    };
    if !name
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphanumeric())
    {
        return None;
    }

    let (description, tail) = match tail.strip_prefix('[') {
        Some(tail) => {
            let end = unescaped(tail, ']')?;
            (unescape(&tail[..end]), &tail[end + 1..])
        }
        None => (String::new(), tail),
    };
    let value = if tail.starts_with("::") || (attached_only && tail.starts_with(':')) {
        Value::Optional
    } else if tail.starts_with(':') {
        Value::Required
    } else {
        Value::None
    };
    let (completions, directory) = match value {
        Value::Required => action(tail),
        _ => (Completions::default(), false),
    };
    Some(Spec {
        name: name.to_string(),
        long,
        completions,
        directory,
        value,
        description,
    })
}

/// Parses a positional spec of `_arguments`, such as `1:branch:(main dev)` or `*:host:_hosts`,
/// into what its action completes, or `None` for any other word.
fn parse_positional(text: &str) -> Option<Completions> {
    let mut rest = text;
    loop {
        if let Some(after) = rest.strip_prefix('*') {
            rest = after;
        } else if rest.starts_with('(') {
            rest = &rest[rest.find(')')? + 1..];
        } else {
            break;
        }
    }
    let rest = rest.trim_start_matches(|c: char| c.is_ascii_digit());
    if !rest.starts_with(':') {
        return None;
    }
    // `*:::message:action` takes the rest of the words; the colons only say how.
    let (completions, _) = action(&format!(":{}", rest.trim_start_matches(':')));
    (!completions.is_empty()).then_some(completions)
}

/// What the action of `:message:action` completes, and whether it completes directories: the
/// values of `(a b c)` or `((a\:"about a" b\:"about b"))`, or a shared function such as
/// `_hosts`. Other actions, such as `->state` or a function of the tool's, complete nothing
/// here.
fn action(tail: &str) -> (Completions, bool) {
    let Some(rest) = tail.strip_prefix(':') else {
        return (Completions::default(), false);
    };
    let Some(end) = unescaped(rest, ':') else {
        return (Completions::default(), false);
    };
    let action = &rest[end + 1..];
    let values = if let Some(items) = action.strip_prefix("((") {
        described_values(items)
    } else if let Some(items) = action.strip_prefix('(') {
        let items = &items[..unescaped(items, ')').unwrap_or(items.len())];
        items.split_whitespace().map(unescape).collect()
    } else {
        let mut words = action.split_whitespace();
        let function = words.next().unwrap_or_default();
        let shared = match completers::zsh_shared(function) {
            // `_files -/` completes directories only.
            Some(Shared::Files) if words.any(|word| word == "-/") => Some(Shared::Directory),
            shared => shared,
        };
        return match shared {
            Some(Shared::Directory) => (Completions::default(), true),
            Some(Shared::Dynamic(dynamic)) => (
                Completions {
                    dynamic: vec![dynamic],
                    ..Completions::default()
                },
                false,
            ),
            Some(Shared::Files) | None => (Completions::default(), false),
        };
    };
    (
        Completions {
            values,
            ..Completions::default()
        },
        false,
    )
}

/// The values of `a\:"about a" b\:"about b"))`: each item's text before its `\:`, skipping
/// the description, which may be quoted and hold spaces.
fn described_values(items: &str) -> Vec<String> {
    let mut values = vec![];
    let mut chars = items.chars().peekable();
    loop {
        while chars.next_if(|c| c.is_whitespace()).is_some() {}
        if chars.peek().is_none_or(|c| *c == ')') {
            return values;
        }
        let mut value = String::new();
        while let Some(c) = chars.next_if(|c| !c.is_whitespace() && *c != ')') {
            match c {
                '\\' if chars.peek() == Some(&':') => break,
                '\\' => value.extend(chars.next()),
                ':' => break,
                _ => value.push(c),
            }
        }
        if !value.is_empty() {
            values.push(value);
        }
        // The description, up to the whitespace after it.
        let mut quoted = false;
        while let Some(c) = chars.next_if(|c| quoted || !(c.is_whitespace() || *c == ')')) {
            match c {
                '"' => quoted = !quoted,
                '\\' => {
                    chars.next();
                }
                _ => {}
            }
        }
    }
}

/// The index of the first `close` that isn't escaped with a backslash.
fn unescaped(text: &str, close: char) -> Option<usize> {
    let mut escaped = false;
    for (index, c) in text.char_indices() {
        match c {
            _ if escaped => escaped = false,
            '\\' => escaped = true,
            _ if c == close => return Some(index),
            _ => {}
        }
    }
    None
}

/// Removes the backslashes `_arguments` uses to escape `:`, `[` and `]`.
fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => out.extend(chars.next()),
            _ => out.push(c),
        }
    }
    out
}

/// A word of the script after quote removal, or a separator.
#[derive(Debug, PartialEq)]
enum Token {
    /// One word. Quote removal can't tell an unquoted `(` or `)` from a quoted one, so the
    /// word says whether it has the ones that start and end an array.
    Word {
        /// The expansions of the word: more than one when it has an unquoted `{a,b}`.
        expansions: Vec<String>,
        /// Whether an unquoted `(` follows `name=`, which starts an array.
        opens_array: bool,
        /// Whether the word ends with an unquoted `)`, which ends an array or a case pattern.
        closes: bool,
    },
    /// A newline, `;`, `&&`, `||` or `&`.
    Separator,
    /// A `|` between commands, or between the alternatives of a case pattern.
    Pipe,
    /// `;;`, `;&` or `;|`, which end a `case` arm.
    ArmEnd,
}

/// Splits a zsh script into words the way zsh does, removing quotes and backslashes and
/// expanding unquoted `{a,b}`. Parameter expansions and command substitutions are kept as
/// written, and `#` at the start of a word starts a comment.
fn tokens(script: &str) -> Vec<Token> {
    let chars = script.chars().collect::<Vec<_>>();
    let mut tokens = vec![];
    let mut word = WordBuilder::default();
    let mut i = 0;

    while let Some(&c) = chars.get(i) {
        let next = chars.get(i + 1).copied();
        match c {
            '\'' => {
                word.started = true;
                i += 1;
                while let Some(&q) = chars.get(i).filter(|q| **q != '\'') {
                    word.push(q);
                    i += 1;
                }
                i += 1;
            }
            '"' => {
                word.started = true;
                i += 1;
                while let Some(&q) = chars.get(i).filter(|q| **q != '"') {
                    if q == '\\' && matches!(chars.get(i + 1), Some('"' | '\\' | '$' | '`')) {
                        i += 1;
                    }
                    word.extend(chars.get(i));
                    i += 1;
                }
                i += 1;
            }
            '$' if next == Some('\'') => {
                word.started = true;
                i += 2;
                while let Some(&q) = chars.get(i).filter(|q| **q != '\'') {
                    if q == '\\' {
                        i += 1;
                    }
                    word.extend(chars.get(i));
                    i += 1;
                }
                i += 1;
            }
            '$' if matches!(next, Some('(' | '{')) => {
                // Kept as written, up to the matching bracket.
                word.started = true;
                let (open, close) = if next == Some('(') {
                    ('(', ')')
                } else {
                    ('{', '}')
                };
                let mut depth = 0usize;
                word.push('$');
                i += 1;
                while let Some(&q) = chars.get(i) {
                    word.push(q);
                    i += 1;
                    if q == open {
                        depth += 1;
                    } else if q == close {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                }
            }
            '\\' => {
                i += 2;
                if next != Some('\n') {
                    word.started = true;
                    word.extend(next.as_ref());
                }
            }
            '#' if !word.started => {
                while chars.get(i).is_some_and(|q| *q != '\n') {
                    i += 1;
                }
            }
            '\n' | ';' => {
                word.finish(&mut tokens);
                if c == ';' && matches!(next, Some(';' | '&' | '|')) {
                    tokens.push(Token::ArmEnd);
                    i += 2;
                } else {
                    tokens.push(Token::Separator);
                    i += 1;
                }
            }
            '&' | '|' if word.parens == 0 => {
                word.finish(&mut tokens);
                tokens.push(match (c, next) {
                    ('|', Some(n)) if n != '|' => Token::Pipe,
                    ('|', None) => Token::Pipe,
                    _ => Token::Separator,
                });
                i += if next == Some(c) { 2 } else { 1 };
            }
            ' ' | '\t' | '\r' if word.parens == 0 => {
                word.finish(&mut tokens);
                i += 1;
            }
            '{' => match brace_alternatives(&chars[i..]) {
                Some((alternatives, len)) => {
                    word.started = true;
                    word.closes = false;
                    word.parts.push(vec![std::mem::take(&mut word.text)]);
                    word.parts.push(alternatives);
                    i += len;
                }
                None => {
                    word.started = true;
                    word.push(c);
                    i += 1;
                }
            },
            _ => {
                word.started = true;
                match c {
                    // `name=(` starts an array, whose items are words of their own.
                    '(' if word.text.ends_with('=') => word.opens_array = true,
                    '(' => word.parens += 1,
                    ')' => word.parens = word.parens.saturating_sub(1),
                    _ => {}
                }
                word.push(c);
                word.closes = c == ')';
                i += 1;
            }
        }
    }
    word.finish(&mut tokens);
    tokens
}

#[derive(Default)]
struct WordBuilder {
    started: bool,
    /// The parts before `text`: plain text, or the alternatives of a brace expansion.
    parts: Vec<Vec<String>>,
    text: String,
    /// Unquoted parentheses still open, as in a case pattern `(add|rm)`.
    parens: usize,
    /// Whether an unquoted `(` follows `name=`.
    opens_array: bool,
    /// Whether the last character added is an unquoted `)`.
    closes: bool,
}

impl WordBuilder {
    fn push(&mut self, c: char) {
        self.text.push(c);
        self.closes = false;
    }

    fn extend(&mut self, c: Option<&char>) {
        if let Some(&c) = c {
            self.push(c);
        }
    }

    fn finish(&mut self, tokens: &mut Vec<Token>) {
        if !self.started {
            return;
        }
        let mut expansions = vec![String::new()];
        for part in self
            .parts
            .drain(..)
            .chain([vec![std::mem::take(&mut self.text)]])
        {
            expansions = expansions
                .iter()
                .flat_map(|prefix| {
                    part.iter()
                        .map(move |alternative| format!("{prefix}{alternative}"))
                })
                .collect();
        }
        tokens.push(Token::Word {
            expansions,
            opens_array: self.opens_array,
            closes: self.closes,
        });
        *self = Self::default();
    }
}

/// Reads an unquoted brace expansion `{a,b}` at the start of `chars`, and returns its
/// alternatives and its length. Anything else, such as a lone `{`, returns `None`.
fn brace_alternatives(chars: &[char]) -> Option<(Vec<String>, usize)> {
    let mut alternatives = vec![String::new()];
    for (index, &c) in chars.iter().enumerate().skip(1) {
        match c {
            '}' => return (alternatives.len() > 1).then_some((alternatives, index + 1)),
            ',' => alternatives.push(String::new()),
            '{' | '\'' | '"' | '\\' | '$' => return None,
            _ if c.is_whitespace() => return None,
            _ => alternatives.last_mut()?.push(c),
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(specs: &[CommandSpec]) -> Vec<String> {
        specs.iter().map(|spec| spec.path.join(" ")).collect()
    }

    #[test]
    fn reads_zsh_style_functions() {
        let script = r#"#compdef git
_git-status () {
  local -a branch_opts
  branch_opts=('(-b --branch)'{-b,--branch}'[show branch and tracking info]')
  _arguments -S -s $endopt \
    '(-s --short --column)'{-s,--short}'[output in short format]' \
    '(-s --short)--porcelain=-[produce machine-readable output]:version:(v1)' \
    '(-u --untracked-files)'{-u-,--untracked-files=-}'[show untracked files]::mode:((no\:"none"
        normal\:"some"))' \
    '--ignore-submodules=[ignore changes to submodules]:when:(all dirty)' \
    '--mode=[pick a mode]:mode:((fast\:"skip the checks" safe\:"run every check"))' \
    '*: :__git_ignore_line_inside_arguments _files'
}

_git-parseRefs () {
  _arguments '--refs[not a subcommand]'
}

__git_setup_log_options () {
  log_options=('--decorate[print out ref names]')
}

_git_commands () {
  local -a main_porcelain_cmds
  main_porcelain_cmds=(
    status:'show the working-tree status'
    add:'add file contents to index')
}

_git () {
  _arguments '(- :)--version[display version information]'
}
"#;
        let specs = parse(script, "git", &[]);
        assert_eq!(names(&specs), ["", "status", "add"]);
        assert_eq!(
            specs[0].flag_tuples(),
            [(Some("version"), None, None, "display version information")]
        );
        let status = &specs[1];
        assert_eq!(status.description, "show the working-tree status");
        assert_eq!(
            status.flag_tuples(),
            [
                (
                    Some("branch"),
                    Some('b'),
                    None,
                    "show branch and tracking info"
                ),
                (Some("short"), Some('s'), None, "output in short format"),
                (
                    Some("ignore-submodules"),
                    None,
                    Some(SyntaxShape::String),
                    "ignore changes to submodules"
                ),
                (Some("mode"), None, Some(SyntaxShape::String), "pick a mode"),
            ]
        );
        assert_eq!(status.flags[2].completions.values, ["all", "dirty"]);
        assert_eq!(status.flags[3].completions.values, ["fast", "safe"]);
        assert_eq!(status.skipped, ["--porcelain", "-u", "--untracked-files"]);
    }

    #[test]
    fn reads_clap_style_functions() {
        let script = r#"#compdef tool
_tool() {
    _arguments "${_arguments_options[@]}" : \
'*-e+[A pattern to search for.]:PATTERN:_default' \
'*--regexp=[A pattern to search for.]:PATTERN:_default' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
":: :_tool_commands" \
"*::: :->tool" \
&& ret=0
    case $state in
    (tool)
        case $line[1] in
            (build|b)
_arguments "${_arguments_options[@]}" : \
'--release[Build in release mode]' \
&& ret=0
;;
            (*)
_arguments '--weird[unknown arm]'
;;
        esac
    ;;
esac
}
_tool_commands() {
    local commands; commands=(
'build:Compile the package' \
'clean:Remove artifacts' \
    )
}
"#;
        let specs = parse(script, "tool", &[]);
        assert_eq!(names(&specs), ["", "build", "b", "clean"]);
        assert_eq!(
            specs[0].flag_tuples(),
            [
                (
                    Some("regexp"),
                    Some('e'),
                    Some(SyntaxShape::String),
                    "A pattern to search for."
                ),
                (
                    Some("help"),
                    Some('h'),
                    None,
                    "Print help (see more with '--help')"
                ),
            ]
        );
        assert_eq!(
            specs[1].flag_tuples(),
            [(Some("release"), None, None, "Build in release mode")]
        );
        assert_eq!(specs[3].description, "Remove artifacts");
        assert_eq!(names(&parse(script, "tool", &["build".into()])), ["build"]);
    }

    #[test]
    fn reads_positional_and_shared_actions() {
        let script = r#"#compdef tool
_tool-connect () {
  _arguments \
    '--jump=[jump host]:host:_hosts' \
    '-C+[run in directory]:directory:_directories' \
    '1:mode:(fast safe)' \
    '*:host:_hosts' \
    '*:: :->rest'
}
"#;
        let specs = parse(script, "tool", &[]);
        let connect = &specs[1];
        assert_eq!(connect.args.values, ["fast", "safe"]);
        assert_eq!(
            connect.args.dynamic,
            [Dynamic::Library(completers::Library::Hosts)]
        );
        assert_eq!(
            connect.flags[0].completions.dynamic,
            [Dynamic::Library(completers::Library::Hosts)]
        );
        assert_eq!(connect.flags[1].arg, Some(SyntaxShape::Directory));
    }

    #[test]
    fn reads_case_patterns_and_described_commands() {
        let script = r#"#compdef tool
_tool() {
    local cmds="(-x --exclusive)"
    case $words[1] in
        add|rm) _arguments '--force[force it]' ;;
    esac
    _arguments '-C+[run in]:dir:_files -/' '-c+[color]:when:(always never)' '--color=[color]:when:(auto)'
}
_tool_commands() {
    local commands; commands=('build:Compile the package (debug)' 'clean:Remove artifacts' 'help:Print help of the given subcommand(s)')
}
"#;
        let specs = parse(script, "tool", &[]);
        assert_eq!(names(&specs), ["", "add", "rm", "build", "clean", "help"]);
        assert_eq!(specs[1].flags[0].long.as_deref(), Some("force"));
        assert_eq!(specs[3].description, "Compile the package (debug)");
        assert_eq!(
            specs[5].description,
            "Print help of the given subcommand(s)"
        );
        let root = &specs[0];
        assert_eq!(root.flags[0].arg, Some(SyntaxShape::Directory));
        assert_eq!(
            (root.flags[1].long.as_deref(), root.flags[1].short),
            (Some("color"), Some('c'))
        );
        assert_eq!(
            root.flags[1].completions.values,
            ["always", "never", "auto"]
        );
    }

    fn word(expansions: &[&str], closes: bool) -> Token {
        Token::Word {
            expansions: expansions.iter().map(|text| text.to_string()).collect(),
            opens_array: false,
            closes,
        }
    }

    #[test]
    fn splits_words_like_zsh() {
        assert_eq!(
            tokens(
                "a '(x y)'{-s,--short}'[it'\\''s]' \"${opts[@]}\" # comment\n(b|c) ;; d|e) 'f)' g=(h"
            ),
            [
                word(&["a"], false),
                word(&["(x y)-s[it's]", "(x y)--short[it's]"], false),
                word(&["${opts[@]}"], false),
                Token::Separator,
                word(&["(b|c)"], true),
                Token::ArmEnd,
                word(&["d"], false),
                Token::Pipe,
                word(&["e)"], true),
                word(&["f)"], false),
                Token::Word {
                    expansions: vec!["g=(h".into()],
                    opens_array: true,
                    closes: false,
                },
            ]
        );
    }
}
