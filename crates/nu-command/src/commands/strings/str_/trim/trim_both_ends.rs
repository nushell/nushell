use super::operate;
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, Value};
pub struct SubCommand;

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str trim"
    }

    fn signature(&self) -> Signature {
        Signature::build("str trim")
            .rest(
                SyntaxShape::ColumnPath,
                "optionally trim text by column paths",
            )
            .named(
                "char",
                SyntaxShape::String,
                "character to trim (default: whitespace)",
                Some('c'),
            )
            .switch(
                "all",
                "trim all characters (default: whitespace)",
                Some('a'),
            )
    }

    fn usage(&self) -> &str {
        "trims text"
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        operate(args, &trim)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Trim whitespace",
                example: "echo 'Nu shell ' | str trim",
                result: Some(vec![Value::from("Nu shell")]),
            },
            Example {
                description: "Trim a specific character",
                example: "echo '=== Nu shell ===' | str trim -c '=' | str trim",
                result: Some(vec![Value::from("Nu shell")]),
            },
            Example {
                description: "Trim all characters",
                example: "echo ' Nu   shell ' | str trim -a",
                result: Some(vec![Value::from("Nu shell")]),
            },
        ]
    }
}
fn trim(s: &str, char_: Option<char>, all_flag: bool) -> String {
    match all_flag {
        false => match char_ {
            None => String::from(s.trim()),
            Some(ch) => String::from(s.trim_matches(ch)),
        },
        true => trim_all(s, char_),
    }
}

fn trim_all(s: &str, char_: Option<char>) -> String {
    let delimiter = char_.unwrap_or(' ');
    let mut buf: Vec<char> = vec![];
    let mut is_delim = false;
    for c in s.chars() {
        match c {
            x if x == delimiter && buf.is_empty() => continue,
            x if x == delimiter => is_delim = true,
            _ => {
                if is_delim {
                    buf.push(delimiter);
                    is_delim = false;
                }
                buf.push(c);
            }
        }
    }
    buf.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::{trim, SubCommand};
    use crate::commands::strings::str_::trim::{action, ActionMode};
    use nu_protocol::row;
    use nu_source::Tag;
    use nu_test_support::value::{int, string, table};

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn trims() {
        let word = string("andres ");
        let expected = string("andres");

        let actual = action(&word, Tag::unknown(), None, false, &trim, ActionMode::Local).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_global() {
        let word = string(" global   ");
        let expected = string("global");

        let actual = action(
            &word,
            Tag::unknown(),
            None,
            false,
            &trim,
            ActionMode::Global,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_ignores_numbers() {
        let number = int(2020);
        let expected = int(2020);

        let actual = action(
            &number,
            Tag::unknown(),
            None,
            false,
            &trim,
            ActionMode::Global,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_row() {
        let row = row!["a".to_string() => string("    c "), " b ".to_string() => string("  d   ")];
        let expected = row!["a".to_string() => string("c"), " b ".to_string() => string("d")];

        let actual = action(&row, Tag::unknown(), None, false, &trim, ActionMode::Global).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_table() {
        let row = table(&[string("  a  "), int(65), string(" d")]);
        let expected = table(&[string("a"), int(65), string("d")]);

        let actual = action(&row, Tag::unknown(), None, false, &trim, ActionMode::Global).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_custom_character_both_ends() {
        let word = string("!#andres#!");
        let expected = string("#andres#");

        let actual = action(
            &word,
            Tag::unknown(),
            Some('!'),
            false,
            &trim,
            ActionMode::Local,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }
    #[test]
    fn trims_all_white_space() {
        let word = string(" Value1 a lot  of  spaces ");
        let expected = string("Value1 a lot of spaces");

        let actual = action(
            &word,
            Tag::unknown(),
            Some(' '),
            true,
            &trim,
            ActionMode::Local,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trims_row_all_white_space() {
        let row = row!["a".to_string() => string("    nu    shell "), " b ".to_string() => string("  b c   d     e  ")];
        let expected =
            row!["a".to_string() => string("nu shell"), " b ".to_string() => string("b c d e")];

        let actual = action(&row, Tag::unknown(), None, true, &trim, ActionMode::Global).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trims_table_all_white_space() {
        let row = table(&[string("  nu      shell   "), int(65), string(" d")]);
        let expected = table(&[string("nu shell"), int(65), string("d")]);

        let actual = action(&row, Tag::unknown(), None, true, &trim, ActionMode::Global).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_all_custom_character() {
        let word = string(".Value1.a.lot..of...dots.");
        let expected = string("Value1.a.lot.of.dots");

        let actual = action(
            &word,
            Tag::unknown(),
            Some('.'),
            true,
            &trim,
            ActionMode::Local,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trims_row_all_custom_character() {
        let row = row!["a".to_string() => string("!!!!nu!!shell!!!"), " b ".to_string() => string("!!b!c!!d!e!!")];
        let expected =
            row!["a".to_string() => string("nu!shell"), " b ".to_string() => string("b!c!d!e")];

        let actual = action(
            &row,
            Tag::unknown(),
            Some('!'),
            true,
            &trim,
            ActionMode::Global,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trims_table_all_custom_character() {
        let row = table(&[string("##nu####shell##"), int(65), string("#d")]);
        let expected = table(&[string("nu#shell"), int(65), string("d")]);

        let actual = action(
            &row,
            Tag::unknown(),
            Some('#'),
            true,
            &trim,
            ActionMode::Global,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }
}
