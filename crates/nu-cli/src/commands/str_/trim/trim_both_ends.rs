use super::operate;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, Value};

pub struct SubCommand;

#[async_trait]
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
    }

    fn usage(&self) -> &str {
        "trims text"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        operate(args, registry, &trim).await
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
        ]
    }
}
fn trim(s: &str, char_: Option<char>) -> String {
    match char_ {
        None => String::from(s.trim()),
        Some(ch) => String::from(s.trim_matches(ch)),
    }
}

#[cfg(test)]
mod tests {
    use super::{trim, SubCommand};
    use crate::commands::str_::trim::action;
    use nu_plugin::test_helpers::value::string;
    use nu_source::Tag;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn trims() {
        let word = string("andres ");
        let expected = string("andres");

        let actual = action(&word, Tag::unknown(), None, &trim).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_custom_character_both_ends() {
        let word = string("!#andres#!");
        let expected = string("#andres#");

        let actual = action(&word, Tag::unknown(), Some('!'), &trim).unwrap();
        assert_eq!(actual, expected);
    }
}
