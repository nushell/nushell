use super::operate;
use crate::prelude::*;
use inflector::cases::screamingsnakecase::to_screaming_snake_case;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, Value};

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str screaming-snake-case"
    }

    fn signature(&self) -> Signature {
        Signature::build("str screaming-snake-case").rest(
            SyntaxShape::ColumnPath,
            "optionally convert text to SCREAMING_SNAKE_CASE by column paths",
        )
    }

    fn usage(&self) -> &str {
        "converts a string to SCREAMING_SNAKE_CASE"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        operate((args), &to_screaming_snake_case).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "convert a string to SCREAMING_SNAKE_CASE",
            example: "echo 'NuShell' | str screaming-snake-case",
            result: Some(vec![Value::from("NU_SHELL")]),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::{to_screaming_snake_case, SubCommand};
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
        let expected = string("THIS_IS_THE_FIRST_CASE");

        let actual = action(&word, Tag::unknown(), &to_screaming_snake_case).unwrap();
        assert_eq!(actual, expected);
    }
    #[test]
    fn snake_case_from_snake() {
        let word = string("this_is_the_second_case");
        let expected = string("THIS_IS_THE_SECOND_CASE");

        let actual = action(&word, Tag::unknown(), &to_screaming_snake_case).unwrap();
        assert_eq!(actual, expected);
    }
}
