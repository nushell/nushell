use super::operate;
use crate::prelude::*;
use inflector::cases::kebabcase::to_kebab_case;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, Value};

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str kebab-case"
    }

    fn signature(&self) -> Signature {
        Signature::build("str kebab-case").rest(
            SyntaxShape::ColumnPath,
            "optionally convert text to kebab-case by column paths",
        )
    }

    fn usage(&self) -> &str {
        "converts a string to kebab-case"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        operate((args), &to_kebab_case).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "convert a string to kebab-case",
            example: "echo 'NuShell' | str kebab-case",
            result: Some(vec![Value::from("nu-shell")]),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::{to_kebab_case, SubCommand};
    use crate::commands::str_::case::action;
    use nu_source::Tag;
    use nu_test_support::value::string;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn kebab_case_from_camel() {
        let word = string("thisIsTheFirstCase");
        let expected = string("this-is-the-first-case");

        let actual = action(&word, Tag::unknown(), &to_kebab_case).unwrap();
        assert_eq!(actual, expected);
    }
    #[test]
    fn kebab_case_from_screaming_snake() {
        let word = string("THIS_IS_THE_SECOND_CASE");
        let expected = string("this-is-the-second-case");

        let actual = action(&word, Tag::unknown(), &to_kebab_case).unwrap();
        assert_eq!(actual, expected);
    }
}
