use super::operate;
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, Value};

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str ltrim"
    }

    fn signature(&self) -> Signature {
        Signature::build("str ltrim")
            .rest(
                SyntaxShape::ColumnPath,
                "optionally trim text starting from the beginning by column paths",
            )
            .named(
                "char",
                SyntaxShape::String,
                "character to trim (default: whitespace)",
                Some('c'),
            )
    }

    fn usage(&self) -> &str {
        "trims whitespace or character from the beginning of text"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        operate((args), &trim_left).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Trim whitespace from the beginning of string",
                example: "echo ' Nu shell ' | str ltrim",
                result: Some(vec![Value::from("Nu shell ")]),
            },
            Example {
                description: "Trim a specific character",
                example: "echo '=== Nu shell ===' | str ltrim -c '='",
                result: Some(vec![Value::from(" Nu shell ===")]),
            },
        ]
    }
}

fn trim_left(s: &str, char_: Option<char>) -> String {
    match char_ {
        None => String::from(s.trim_start()),
        Some(ch) => String::from(s.trim_start_matches(ch)),
    }
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::{trim_left, SubCommand};
    use crate::commands::str_::trim::{action, ActionMode};
    use nu_protocol::row;
    use nu_source::Tag;
    use nu_test_support::value::{int, string, table};

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn trims_whitespace_from_left() {
        let word = string(" andres ");
        let expected = string("andres ");

        let actual = action(&word, Tag::unknown(), None, &trim_left, ActionMode::Local).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_left_global() {
        let word = string(" global   ");
        let expected = string("global   ");

        let actual = action(&word, Tag::unknown(), None, &trim_left, ActionMode::Global).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_left_ignores_numbers() {
        let number = int(2020);
        let expected = int(2020);

        let actual = action(
            &number,
            Tag::unknown(),
            None,
            &trim_left,
            ActionMode::Global,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_left_row() {
        let row = row!["a".to_string() => string("    c "), " b ".to_string() => string("  d   ")];
        let expected = row!["a".to_string() => string("c "), " b ".to_string() => string("d   ")];

        let actual = action(&row, Tag::unknown(), None, &trim_left, ActionMode::Global).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_left_table() {
        let row = table(&[string("  a  "), int(65), string(" d")]);
        let expected = table(&[string("a  "), int(65), string("d")]);

        let actual = action(&row, Tag::unknown(), None, &trim_left, ActionMode::Global).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_custom_chars_from_left() {
        let word = string("!!! andres !!!");
        let expected = string(" andres !!!");

        let actual = action(
            &word,
            Tag::unknown(),
            Some('!'),
            &trim_left,
            ActionMode::Local,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }
}
