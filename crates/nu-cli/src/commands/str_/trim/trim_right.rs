use super::operate;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, Value};

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str rtrim"
    }

    fn signature(&self) -> Signature {
        Signature::build("str rtrim")
            .rest(
                SyntaxShape::ColumnPath,
                "optionally trim text starting from the end by column paths",
            )
            .named(
                "char",
                SyntaxShape::String,
                "character to trim (default: whitespace)",
                Some('c'),
            )
    }

    fn usage(&self) -> &str {
        "trims whitespace or character from the end of text"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        operate(args, registry, &trim_right).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Trim whitespace from the end of string",
                example: "echo ' Nu shell ' | str rtrim",
                result: Some(vec![Value::from(" Nu shell")]),
            },
            Example {
                description: "Trim a specific character",
                example: "echo '=== Nu shell ===' | str rtrim -c '='",
                result: Some(vec![Value::from("=== Nu shell ")]),
            },
        ]
    }
}

fn trim_right(s: &str, char_: Option<char>) -> String {
    match char_ {
        None => String::from(s.trim_end()),
        Some(ch) => String::from(s.trim_end_matches(ch)),
    }
}

#[cfg(test)]
mod tests {
    use super::{trim_right, SubCommand};
    use crate::commands::str_::trim::{action, ActionMode};
    use nu_plugin::{
        row,
        test_helpers::value::{int, string, table},
    };
    use nu_source::Tag;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn trims_whitespace_from_right() {
        let word = string(" andres ");
        let expected = string(" andres");

        let actual = action(&word, Tag::unknown(), None, &trim_right, ActionMode::Local).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_right_global() {
        let word = string(" global   ");
        let expected = string(" global");

        let actual = action(&word, Tag::unknown(), None, &trim_right, ActionMode::Global).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_right_ignores_numbers() {
        let number = int(2020);
        let expected = int(2020);

        let actual = action(
            &number,
            Tag::unknown(),
            None,
            &trim_right,
            ActionMode::Global,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_row() {
        let row = row!["a".to_string() => string("    c "), " b ".to_string() => string("  d   ")];
        let expected = row!["a".to_string() => string("    c"), " b ".to_string() => string("  d")];

        let actual = action(&row, Tag::unknown(), None, &trim_right, ActionMode::Global).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_table() {
        let row = table(&[string("  a  "), int(65), string(" d")]);
        let expected = table(&[string("  a"), int(65), string(" d")]);

        let actual = action(&row, Tag::unknown(), None, &trim_right, ActionMode::Global).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_custom_chars_from_right() {
        let word = string("#@! andres !@#");
        let expected = string("#@! andres !@");

        let actual = action(
            &word,
            Tag::unknown(),
            Some('#'),
            &trim_right,
            ActionMode::Local,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }
}
