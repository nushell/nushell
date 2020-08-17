use super::operate;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use inflector::cases::pascalcase::to_pascal_case;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, Value};

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str pascal-case"
    }

    fn signature(&self) -> Signature {
        Signature::build("str pascal-case").rest(
            SyntaxShape::ColumnPath,
            "optionally convert text to PascalCase by column paths",
        )
    }

    fn usage(&self) -> &str {
        "converts a string to PascalCase"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        operate(args, registry, &to_pascal_case).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "convert a string to PascalCase",
            example: "echo 'nu-shell' | str pascal-case",
            result: Some(vec![Value::from("NuShell")]),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::{to_pascal_case, SubCommand};
    use crate::commands::str_::case::action;
    use nu_plugin::test_helpers::value::string;
    use nu_source::Tag;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn pascal_case_from_kebab() {
        let word = string("this-is-the-first-case");
        let expected = string("ThisIsTheFirstCase");

        let actual = action(&word, Tag::unknown(), &to_pascal_case).unwrap();
        assert_eq!(actual, expected);
    }
    #[test]
    fn pascal_case_from_snake() {
        let word = string("this_is_the_second_case");
        let expected = string("ThisIsTheSecondCase");

        let actual = action(&word, Tag::unknown(), &to_pascal_case).unwrap();
        assert_eq!(actual, expected);
    }
}
