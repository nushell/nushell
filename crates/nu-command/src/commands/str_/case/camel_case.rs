use super::operate;
use crate::prelude::*;
use inflector::cases::camelcase::to_camel_case;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, Value};

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str camel-case"
    }

    fn signature(&self) -> Signature {
        Signature::build("str camel-case").rest(
            SyntaxShape::ColumnPath,
            "optionally convert text to camelCase by column paths",
        )
    }

    fn usage(&self) -> &str {
        "converts a string to camelCase"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        operate((args), &to_camel_case).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "convert a string to camelCase",
            example: "echo 'NuShell' | str camel-case",
            result: Some(vec![Value::from("nuShell")]),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::{to_camel_case, SubCommand};
    use crate::commands::str_::case::action;
    use nu_source::Tag;
    use nu_test_support::value::string;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn camel_case_from_kebab() {
        let word = string("this-is-the-first-case");
        let expected = string("thisIsTheFirstCase");

        let actual = action(&word, Tag::unknown(), &to_camel_case).unwrap();
        assert_eq!(actual, expected);
    }
    #[test]
    fn camel_case_from_snake() {
        let word = string("this_is_the_second_case");
        let expected = string("thisIsTheSecondCase");

        let actual = action(&word, Tag::unknown(), &to_camel_case).unwrap();
        assert_eq!(actual, expected);
    }
}
