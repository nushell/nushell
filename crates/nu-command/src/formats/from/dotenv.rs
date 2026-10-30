use nu_engine::command_prelude::*;
use nu_engine::env::env_to_string;
use nu_protocol::{
    engine::{EnvName, env_var_eq},
    shell_error::generic::GenericError,
};
use std::collections::HashMap;

#[derive(Clone)]
pub struct FromDotenv;

impl Command for FromDotenv {
    fn name(&self) -> &str {
        "from dotenv"
    }

    fn signature(&self) -> Signature {
        Signature::build("from dotenv")
            .input_output_types(vec![(Type::String, Type::record())])
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Parse text as a .env file and create a record."
    }

    fn extra_description(&self) -> &str {
        "Each line is `KEY=value`, optionally starting with `export`. Values can be unquoted, \
single-quoted or double-quoted, and quoted values can span lines. In unquoted and double-quoted \
values, `${NAME}` and `$NAME` expand to an earlier key of the file, or else to the environment \
variable; names ignore case, as in `$env`, so `a=2` after `A=1` sets `A` again. A bare `$NAME` \
is letters, digits and `_`, so `$HOST.local` reads HOST, and a `$` before anything else, as in \
`$5` or `$(command)`, stays as written: commands aren't run. Single quotes or `\\$` keep a `$`. As \
in sh, `${NAME:-word}` gives `word` when NAME is unset or empty and `${NAME-word}` only when it's \
unset, while `${NAME:+word}` gives `word` when NAME is set and not empty and `${NAME+word}` when \
it's set at all, so a key holding `-` can't be referenced: `${MY-VAR}` gives MY, or `VAR` when \
it's unset. A line starting with `#` is a comment, and so is a `#` after a blank in an unquoted value. `open` \
reads files named like `.env`, `.env.local` or `prod.env` with this command, and `load-env` \
loads the record into the environment."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["env", "environment", "dotenv"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let (text, span, metadata) = input.collect_string_strict(head)?;
        let entries = parse(&text).map_err(|err| err.into_shell_error(text.clone(), span))?;

        let mut values: HashMap<EnvName, String> = HashMap::new();
        let mut record = Record::new();
        for entry in entries {
            let value = expand(&entry.parts, |name| {
                match values.get(&EnvName::from(name)) {
                    Some(value) => Ok(Some(value.clone())),
                    None => env_text(engine_state, stack, name),
                }
            })?;
            let seen = values
                .insert(EnvName::from(entry.key.as_str()), value.clone())
                .is_some();
            let value = Value::string(value, head);
            // As in `$env`, a key set again keeps the spelling it was first set with.
            if seen
                && let Some((_, old)) = record
                    .iter_mut()
                    .find(|(key, _)| env_var_eq(key, &entry.key))
            {
                *old = value;
            } else {
                record.push(entry.key, value);
            }
        }

        Ok(Value::record(record, head)
            .into_pipeline_data_with_metadata(metadata.map(|md| md.with_content_type(None))))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Read a .env file into a record.",
                example: r#""PORT=8080
export NAME='my app'
# a comment
GREETING=\"hello ${NAME}\"" | from dotenv"#,
                result: Some(Value::test_record(record! {
                    "PORT" => Value::test_string("8080"),
                    "NAME" => Value::test_string("my app"),
                    "GREETING" => Value::test_string("hello my app"),
                })),
            },
            Example {
                description: "Use a default for a variable that isn't set.",
                example: r#""URL=http://${APP_HOST:-localhost}:8080" | from dotenv"#,
                result: Some(Value::test_record(record! {
                    "URL" => Value::test_string("http://localhost:8080"),
                })),
            },
            Example {
                description: "Read a .env file from disk.",
                example: "open .env",
                result: None,
            },
            Example {
                description: "Load a .env file into the environment.",
                example: "open .env | load-env",
                result: None,
            },
        ]
    }
}

/// `$env.NAME` as text, the way an external command sees it: `None` when it's unset, or when it
/// holds a value with no `to_string` conversion, such as a record.
fn env_text(
    engine_state: &EngineState,
    stack: &Stack,
    name: &str,
) -> Result<Option<String>, ShellError> {
    // `$env.ENV_CONVERSIONS` is keyed by the variable's own spelling.
    let name = stack.get_env_var_name(engine_state, name);
    let Some(value) = stack.get_env_var(engine_state, &name) else {
        return Ok(None);
    };
    match env_to_string(&name, value, engine_state, stack) {
        Ok(text) => Ok(Some(text)),
        Err(ShellError::EnvVarNotAString { .. }) => Ok(None),
        Err(err) => Err(err),
    }
}

/// Whether a file's name says it holds .env text, as `.env`, `.env.local` and `prod.env` do.
/// `open` and `save` go by it, since `.env` has no extension and `.env.local`'s isn't a format.
pub(crate) fn is_dotenv_file(name: &str) -> bool {
    // Compared as bytes, so a name in any case needs no lowercased copy.
    let name = name.as_bytes();
    let end = name.len().saturating_sub(4);
    name.get(..5)
        .is_some_and(|start| start.eq_ignore_ascii_case(b".env."))
        || name[end..].eq_ignore_ascii_case(b".env")
}

/// Whether `name` can be a .env variable name: letters, digits, `_`, `.` and `-`, starting with
/// a letter or `_`.
pub(crate) fn is_key(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') && name.chars().all(is_key_char)
}

fn is_key_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-')
}

/// One `KEY=value` line of a .env file.
#[derive(Debug, PartialEq)]
struct Entry {
    key: String,
    parts: Vec<Part>,
}

/// A piece of a value: text as written, or a `${NAME...}` to expand.
#[derive(Debug, PartialEq)]
enum Part {
    Text(String),
    Var { name: String, op: Op },
}

/// What a `${NAME...}` gives, as in sh.
#[derive(Debug, PartialEq)]
enum Op {
    /// `${NAME}`: the value, or nothing when NAME is unset.
    Value,
    /// `${NAME-word}`: `word` when NAME is unset, else the value. With `colon`, as in
    /// `${NAME:-word}`, an empty value takes `word` too.
    Default { word: String, colon: bool },
    /// `${NAME+word}`: `word` when NAME is set, else nothing. With `colon`, as in
    /// `${NAME:+word}`, an empty value gives nothing too.
    Alternative { word: String, colon: bool },
}

/// The errors for a line that isn't an assignment, which [`ParseError::into_shell_error`] explains.
const EXPECTED_NAME: &str = "expected a variable name";
const EXPECTED_EQUALS: &str = "expected `=` after the variable name";

/// A .env syntax error at a byte range of the text.
#[derive(Debug, PartialEq)]
struct ParseError {
    start: usize,
    end: usize,
    msg: &'static str,
}

impl ParseError {
    fn into_shell_error(self, text: String, span: Span) -> ShellError {
        let error =
            GenericError::new("Error while parsing .env text", self.msg, span).with_inner([
                ShellError::OutsideSpannedLabeledError {
                    src: text,
                    error: "Error while parsing .env text".into(),
                    msg: self.msg.into(),
                    span: Span::new(self.start, self.end),
                },
            ]);
        // A file named `.env` can be a shell script, as autoenv's are.
        ShellError::Generic(if [EXPECTED_NAME, EXPECTED_EQUALS].contains(&self.msg) {
            error.with_help(
                "a .env file holds `KEY=value` lines; a shell script, such as an autoenv `.env` \
                 file, isn't one",
            )
        } else {
            error
        })
    }
}

/// Reads the assignments of a .env file, in order.
fn parse(text: &str) -> Result<Vec<Entry>, ParseError> {
    // A byte order mark is skipped rather than cut off, so the offsets still point into `text`.
    let pos = if text.starts_with('\u{feff}') {
        '\u{feff}'.len_utf8()
    } else {
        0
    };
    let mut parser = Parser { text, pos };
    let mut entries = vec![];
    while parser.pos < text.len() {
        parser.skip_blanks();
        match parser.peek() {
            None => break,
            Some('\n' | '\r') => parser.pos += 1,
            Some('#') => parser.skip_line(),
            Some(_) => entries.push(parser.entry()?),
        }
    }
    Ok(entries)
}

/// Joins the parts of a value, looking each `${NAME}` up with `lookup`.
fn expand(
    parts: &[Part],
    mut lookup: impl FnMut(&str) -> Result<Option<String>, ShellError>,
) -> Result<String, ShellError> {
    let mut value = String::new();
    for part in parts {
        match part {
            Part::Text(text) => value.push_str(text),
            Part::Var { name, op } => {
                let found = lookup(name)?;
                // With `:`, an empty value counts as unset.
                let set = |colon: bool| {
                    found
                        .as_ref()
                        .is_some_and(|found| !(colon && found.is_empty()))
                };
                match op {
                    Op::Default { word, colon } if !set(*colon) => value.push_str(word),
                    Op::Value | Op::Default { .. } => {
                        value.push_str(found.as_deref().unwrap_or_default())
                    }
                    Op::Alternative { word, colon } if set(*colon) => value.push_str(word),
                    Op::Alternative { .. } => {}
                }
            }
        }
    }
    Ok(value)
}

/// Reads .env text one assignment at a time.
struct Parser<'a> {
    text: &'a str,
    /// The byte offset of the next character to read.
    pos: usize,
}

impl Parser<'_> {
    /// The next character, without reading it.
    fn peek(&self) -> Option<char> {
        self.text[self.pos..].chars().next()
    }

    /// The text not read yet.
    fn rest(&self) -> &str {
        &self.text[self.pos..]
    }

    /// Reads past spaces and tabs.
    fn skip_blanks(&mut self) {
        let rest = self.rest();
        self.pos += rest.len() - rest.trim_start_matches([' ', '\t']).len();
    }

    /// Reads up to the end of the line, stopping before the line break.
    fn skip_line(&mut self) {
        self.pos = self
            .rest()
            .find('\n')
            .map_or(self.text.len(), |i| self.pos + i);
    }

    /// An error from `start` to the end of the current line.
    fn error(&self, start: usize, msg: &'static str) -> ParseError {
        let end = self
            .rest()
            .find(['\r', '\n'])
            .map_or(self.text.len(), |i| self.pos + i);
        ParseError {
            start,
            end: end.max(start),
            msg,
        }
    }

    /// Reads one `[export] KEY=value` assignment and any comment after it.
    fn entry(&mut self) -> Result<Entry, ParseError> {
        if let Some(after) = self.rest().strip_prefix("export")
            && after.starts_with([' ', '\t'])
        {
            self.pos += "export".len();
            self.skip_blanks();
        }

        let key_len = self
            .rest()
            .find(|c: char| !is_key_char(c))
            .unwrap_or(self.rest().len());
        let key = &self.rest()[..key_len];
        if !is_key(key) {
            return Err(self.error(self.pos, EXPECTED_NAME));
        }
        let key = key.to_string();
        let key_start = self.pos;
        self.pos += key_len;
        self.skip_blanks();
        if self.peek() != Some('=') {
            return Err(ParseError {
                start: key_start,
                end: key_start + key_len,
                msg: EXPECTED_EQUALS,
            });
        }
        self.pos += 1;
        let value_start = self.pos;
        self.skip_blanks();

        let parts = match self.peek() {
            Some('\'') => self.single_quoted()?,
            Some('"') => self.double_quoted()?,
            // In `KEY= # note` the `#` follows a blank, so it starts a comment.
            Some('#') if self.pos > value_start => vec![],
            _ => self.unquoted()?,
        };
        self.skip_blanks();
        match self.peek() {
            None | Some('\n' | '\r') => {}
            Some('#') => self.skip_line(),
            Some(_) => return Err(self.error(self.pos, "unexpected text after the value")),
        }
        Ok(Entry { key, parts })
    }

    /// Reads a `'...'` value, which is taken as written.
    fn single_quoted(&mut self) -> Result<Vec<Part>, ParseError> {
        let open = self.pos;
        self.pos += 1;
        let Some(len) = self.rest().find('\'') else {
            return Err(self.error(open, "missing closing `'`"));
        };
        // A file saved on Windows breaks lines with `\r\n`; the value gets `\n`.
        let text = self.rest()[..len].replace("\r\n", "\n");
        self.pos += len + 1;
        Ok(vec![Part::Text(text)])
    }

    /// Reads a `"..."` value, with backslash escapes and `${NAME}`.
    fn double_quoted(&mut self) -> Result<Vec<Part>, ParseError> {
        let open = self.pos;
        self.pos += 1;
        let mut parts = vec![];
        let mut text = String::new();
        loop {
            let Some(c) = self.peek() else {
                return Err(self.error(open, "missing closing `\"`"));
            };
            self.pos += c.len_utf8();
            match c {
                '"' => break,
                '\\' => {
                    let Some(escaped) = self.peek() else {
                        return Err(self.error(open, "missing closing `\"`"));
                    };
                    self.pos += escaped.len_utf8();
                    push_escaped(&mut text, escaped);
                }
                // A file saved on Windows breaks lines with `\r\n`; the value gets `\n`.
                '\r' if self.rest().starts_with('\n') => {}
                '$' if self.rest().starts_with('{') => {
                    let quote = self.closing_quote();
                    self.variable(quote, true, &mut parts, &mut text)?;
                }
                '$' => {
                    if !self.bare_variable(&mut parts, &mut text) {
                        text.push(c);
                    }
                }
                _ => text.push(c),
            }
        }
        if !text.is_empty() {
            parts.push(Part::Text(text));
        }
        Ok(parts)
    }

    /// Where the double-quoted value being read ends: at its first `"` that no `\` escapes.
    fn closing_quote(&self) -> usize {
        let mut chars = self.rest().char_indices();
        while let Some((i, c)) = chars.next() {
            match c {
                '\\' => {
                    chars.next();
                }
                '"' => return self.pos + i,
                _ => {}
            }
        }
        self.text.len()
    }

    /// Reads an unquoted value up to the end of the line or a comment, trimmed, with `${NAME}`
    /// and `$NAME`.
    fn unquoted(&mut self) -> Result<Vec<Part>, ParseError> {
        let line_end = self
            .rest()
            .find(['\r', '\n'])
            .map_or(self.text.len(), |i| self.pos + i);
        // A `#` after a blank starts a comment.
        let line = &self.text[self.pos..line_end];
        let value_end = line
            .char_indices()
            .find(|&(i, c)| c == '#' && line[..i].ends_with([' ', '\t']))
            .map_or(line_end, |(i, _)| self.pos + i);
        let value_end = self.pos + self.text[self.pos..value_end].trim_end().len();

        let mut parts = vec![];
        let mut text = String::new();
        while self.pos < value_end {
            let Some(c) = self.peek() else { break };
            self.pos += c.len_utf8();
            // An unclosed `${` stays as text.
            if c == '$'
                && self.rest().starts_with('{')
                && self.text[self.pos..value_end].contains('}')
            {
                self.variable(value_end, false, &mut parts, &mut text)?;
                continue;
            }
            if c == '$' && self.bare_variable(&mut parts, &mut text) {
                continue;
            }
            text.push(c);
        }
        if !text.is_empty() {
            parts.push(Part::Text(text));
        }
        Ok(parts)
    }

    /// Reads `{NAME}` or `{NAME<op>word}` after a `$`, with the closing `}` before `end`. In a
    /// `quoted` value, the word's `\` escapes are read as in the rest of the value. The text
    /// read so far goes into `parts` first.
    fn variable(
        &mut self,
        end: usize,
        quoted: bool,
        parts: &mut Vec<Part>,
        text: &mut String,
    ) -> Result<(), ParseError> {
        let source = self.text;
        let open = self.pos - 1;
        let Some(len) = source[self.pos..end].find('}') else {
            return Err(self.error(open, "missing closing `}`"));
        };
        let close = self.pos + len + 1;
        let error = |msg| ParseError {
            start: open,
            end: close,
            msg,
        };
        let inner = &source[self.pos + 1..close - 1];
        let name_len = inner
            .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '.')))
            .unwrap_or(inner.len());
        let (name, rest) = inner.split_at(name_len);
        if !is_key(name) {
            return Err(error("expected a variable name after `${`"));
        }
        if rest.contains("${") {
            return Err(error("a `${...}` inside another isn't supported"));
        }
        let (colon, op) = rest
            .strip_prefix(':')
            .map_or((false, rest), |op| (true, op));
        let word = |word: &str| {
            if quoted {
                let mut text = String::new();
                let mut chars = word.chars();
                while let Some(c) = chars.next() {
                    match c {
                        '\\' => match chars.next() {
                            Some(escaped) => push_escaped(&mut text, escaped),
                            None => text.push(c),
                        },
                        c => text.push(c),
                    }
                }
                text
            } else {
                word.to_string()
            }
        };
        let op = if rest.is_empty() {
            Op::Value
        } else if let Some(text) = op.strip_prefix('-') {
            Op::Default {
                word: word(text),
                colon,
            }
        } else if let Some(text) = op.strip_prefix('+') {
            Op::Alternative {
                word: word(text),
                colon,
            }
        } else {
            // sh's `${NAME:?message}` and `${NAME:=word}` fail or assign, which a .env file can't.
            return Err(error(
                "expected `}`, `:-`, `-`, `:+` or `+` after the variable name",
            ));
        };
        push_var(parts, text, name, op);
        self.pos = close;
        Ok(())
    }

    /// Reads the name of a `$NAME` after its `$`: a letter or `_`, then letters, digits and `_`,
    /// as in sh. The text read so far goes into `parts` first. Reads nothing and returns false
    /// when no name follows, so `$5` and `$(command)` stay as written.
    fn bare_variable(&mut self, parts: &mut Vec<Part>, text: &mut String) -> bool {
        let rest = self.rest();
        if !rest.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') {
            return false;
        }
        let len = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(rest.len());
        push_var(parts, text, &rest[..len], Op::Value);
        self.pos += len;
        true
    }
}

/// Appends a `${NAME}` or `$NAME` to `parts`, after the text read before it.
fn push_var(parts: &mut Vec<Part>, text: &mut String, name: &str, op: Op) {
    if !text.is_empty() {
        parts.push(Part::Text(std::mem::take(text)));
    }
    parts.push(Part::Var {
        name: name.to_string(),
        op,
    });
}

/// Appends what `\` followed by `c` stands for in a double-quoted value.
fn push_escaped(text: &mut String, c: char) {
    match c {
        'n' => text.push('\n'),
        't' => text.push('\t'),
        'r' => text.push('\r'),
        '"' | '\\' | '$' => text.push(c),
        _ => {
            text.push('\\');
            text.push(c);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(text: &str) -> Vec<(String, String)> {
        let entries = parse(text).expect("parses");
        let mut found: HashMap<String, String> = HashMap::new();
        entries
            .iter()
            .map(|entry| {
                let value =
                    expand(&entry.parts, |name| Ok(found.get(name).cloned())).expect("expands");
                found.insert(entry.key.clone(), value.clone());
                (entry.key.clone(), value)
            })
            .collect()
    }

    fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
        list.iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn error(text: &str) -> &'static str {
        parse(text).expect_err("fails").msg
    }

    #[test]
    fn examples_work_as_expected() -> nu_test_support::Result {
        nu_test_support::test().examples(FromDotenv)
    }

    #[test]
    fn dotenv_file_names_match_in_any_case() {
        for name in [
            ".env",
            ".ENV",
            ".env.local",
            ".Env.Production",
            "prod.env",
            "Grüße.ENV",
        ] {
            assert!(is_dotenv_file(name), "{name}");
        }
        for name in ["env", ".envrc", "a.envx", "", "é"] {
            assert!(!is_dotenv_file(name), "{name}");
        }
    }

    #[test]
    fn reads_quoting_comments_and_export() {
        let text =
            "# comment\n\nA=1\nexport B = two words  # trailing\nC='a # b'\nD=\"x\\ny\"\r\nE=\n";
        assert_eq!(
            values(text),
            pairs(&[
                ("A", "1"),
                ("B", "two words"),
                ("C", "a # b"),
                ("D", "x\ny"),
                ("E", "")
            ])
        );
    }

    #[test]
    fn a_comment_after_the_equals_sign_leaves_the_value_empty() {
        assert_eq!(
            values("A= # note\nB=#not a comment\n"),
            pairs(&[("A", ""), ("B", "#not a comment")])
        );
    }

    #[test]
    fn keys_can_hold_hyphens() {
        assert_eq!(values("MY-VAR=1\n"), pairs(&[("MY-VAR", "1")]));
        assert_eq!(error("-A=1\n"), "expected a variable name");
        // `-` starts a default, so the key can't be referenced.
        assert_eq!(
            values("MY-VAR=1\nB=${MY-VAR}\n"),
            pairs(&[("MY-VAR", "1"), ("B", "VAR")])
        );
    }

    #[test]
    fn referenced_names_follow_the_key_rules() {
        assert_eq!(error("A=${1A}\n"), "expected a variable name after `${`");
        assert_eq!(
            error("A=\"${.A}\"\n"),
            "expected a variable name after `${`"
        );
    }

    #[test]
    fn a_default_word_in_double_quotes_reads_escapes() {
        // Unquoted, the word stays as written.
        let text = "A=\"${B:-a\\\"x\\\"\\t}\"\nC=${B:-\\\"y\\\"}\n";
        assert_eq!(
            values(text),
            pairs(&[("A", "a\"x\"\t"), ("C", "\\\"y\\\"")])
        );
    }

    #[test]
    fn quoted_values_span_lines() {
        assert_eq!(
            values("KEY=\"line one\nline two\"\nNEXT='a\nb'\n"),
            pairs(&[("KEY", "line one\nline two"), ("NEXT", "a\nb")])
        );
    }

    #[test]
    fn windows_line_breaks_become_newlines() {
        assert_eq!(
            values("A=\"x\r\ny\"\r\nB='p\r\nq'\r\nC=\"r\\rs\"\r\n"),
            pairs(&[("A", "x\ny"), ("B", "p\nq"), ("C", "r\rs")])
        );
    }

    #[test]
    fn a_byte_order_mark_is_skipped() {
        assert_eq!(values("\u{feff}A=1\n"), pairs(&[("A", "1")]));
        assert_eq!(
            parse("\u{feff}1A=1\n"),
            Err(ParseError {
                start: 3,
                end: 7,
                msg: "expected a variable name"
            })
        );
    }

    #[test]
    fn expands_braced_and_bare_names() {
        let text = "A=x\nB=${A}/$A\nC=\"${MISSING:-d}${A:-unused}\"\nD='${A}'\n";
        assert_eq!(
            values(text),
            pairs(&[("A", "x"), ("B", "x/x"), ("C", "dx"), ("D", "${A}")])
        );
    }

    #[test]
    fn a_bare_name_is_letters_digits_and_underscores() {
        let text = "A_1=x\nB=$A_1.local-$A_1\nC=\"[$A_1]\"\nD=$MISSING!\n";
        assert_eq!(
            values(text),
            pairs(&[("A_1", "x"), ("B", "x.local-x"), ("C", "[x]"), ("D", "!")])
        );
    }

    #[test]
    fn a_dollar_before_anything_else_stays_as_written() {
        let text = "A=$5 $ $(whoami) $\nB=\"$5 $(whoami) $\"\nC='$A'\nD=\"\\$A\"\n";
        assert_eq!(
            values(text),
            pairs(&[
                ("A", "$5 $ $(whoami) $"),
                ("B", "$5 $(whoami) $"),
                ("C", "$A"),
                ("D", "$A")
            ])
        );
    }

    #[test]
    fn an_empty_value_takes_the_default() {
        assert_eq!(values("A=\nB=${A:-d}\n"), pairs(&[("A", ""), ("B", "d")]));
    }

    #[test]
    fn expands_the_sh_operators() {
        let text = "E=\nS=s\nV=${E-a}${E:-b}${E+c}${E:+d}${U-e}${U+f}${S:+g}\n";
        assert_eq!(values(text), pairs(&[("E", ""), ("S", "s"), ("V", "bceg")]));
    }

    #[test]
    fn refuses_operators_a_dotenv_file_cant_run() {
        let msg = "expected `}`, `:-`, `-`, `:+` or `+` after the variable name";
        assert_eq!(error("A=${B:?missing}\n"), msg);
        assert_eq!(error("A=\"${B:=d}\"\n"), msg);
        assert_eq!(error("A=${:-d}\n"), "expected a variable name after `${`");
        assert_eq!(
            error("A=${B:-${C}}\n"),
            "a `${...}` inside another isn't supported"
        );
    }

    #[test]
    fn a_brace_after_the_closing_quote_doesnt_close_a_variable() {
        assert_eq!(error("A=\"${B\"\nC=}\n"), "missing closing `}`");
    }

    #[test]
    fn reports_where_a_line_goes_wrong() {
        assert_eq!(
            parse("A=1\nnot an assignment\n"),
            Err(ParseError {
                start: 4,
                end: 7,
                msg: "expected `=` after the variable name"
            })
        );
        assert!(parse("A=\"open\n").is_err());
        assert!(parse("A='x' extra\n").is_err());
        assert!(parse("1A=x\n").is_err());
    }
}
