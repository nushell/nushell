use super::operate;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
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

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        operate(args, registry, &trim_left).await
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
    use super::{trim_left, SubCommand};
    use crate::commands::str_::trim::action;
    use nu_plugin::test_helpers::value::string;
    use nu_source::Tag;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn trims_whitespace_from_left() {
        let word = string(" andres ");
        let expected = string("andres ");

        let actual = action(&word, Tag::unknown(), None, &trim_left).unwrap();
        assert_eq!(actual, expected);
    }
    #[test]
    fn trims_custom_chars_from_left() {
        let word = string("!!! andres !!!");
        let expected = string(" andres !!!");

        let actual = action(&word, Tag::unknown(), Some('!'), &trim_left).unwrap();
        assert_eq!(actual, expected);
    }
}
