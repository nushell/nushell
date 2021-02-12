use super::operate;
use crate::prelude::*;
use inflector::cases::snakecase::to_snake_case;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, Value};

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str snake-case"
    }

    fn signature(&self) -> Signature {
        Signature::build("str snake-case").rest(
            SyntaxShape::ColumnPath,
            "optionally convert text to snake_case by column paths",
        )
    }

    fn usage(&self) -> &str {
        "converts a string to snake_case"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        operate((args), &to_snake_case).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "convert a string to snake_case",
            example: "echo 'NuShell' | str snake-case",
            result: Some(vec![Value::from("nu_shell")]),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::{to_snake_case, SubCommand};
    use crate::commands::str_::case::action;
    use nu_source::Tag;
    use nu_test_support::value::string;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn snake_case_from_kebab() {
        let word = string("this-is-the-first-case");
        let expected = string("this_is_the_first_case");

        let actual = action(&word, Tag::unknown(), &to_snake_case).unwrap();
        assert_eq!(actual, expected);
    }
    #[test]
    fn snake_case_from_camel() {
        let word = string("thisIsTheSecondCase");
        let expected = string("this_is_the_second_case");

        let actual = action(&word, Tag::unknown(), &to_snake_case).unwrap();
        assert_eq!(actual, expected);
    }
}
