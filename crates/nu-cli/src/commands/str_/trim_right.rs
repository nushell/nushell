use crate::commands::str_::trim_base::operate;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{
    Signature, SyntaxShape, Value,
};

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str rtrim"
    }

    fn signature(&self) -> Signature {
        Signature::build("str ltrim")
            .rest(
                SyntaxShape::ColumnPath,
                "optionally trim text from right by column paths",
            )
            .named(
                "char",
                SyntaxShape::String,
                "character to trim (default: whitespace)",
                Some('c'),
            )
    }

    fn usage(&self) -> &str {
        "trims text from right side"
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
                description: "Trim whitespace",
                example: "echo '  Nu shell ' | str rtrim",
                result: Some(vec![Value::from(" Nu shell")]),
            },
            Example {
                description: "Trim a specific character",
                example: "echo '=== Nu shell ===' | str rtrim -c '=' | str trim",
                result: Some(vec![Value::from("=== Nu shell")]),
            },
        ]
    }
}

fn trim_right(s: &String, char_: Option<char>) -> String {
    match char_ {
        None => String::from(s.trim_end()),
        Some(ch) => String::from(s.trim_end_matches(ch))
    }
}

#[cfg(test)]
mod tests {
    use crate::commands::str_::trim_base::action;
    use super::{trim_right, SubCommand};
    use nu_plugin::test_helpers::value::string;
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

        let actual = action(&word, Tag::unknown(), None, &trim_right).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_custom_chars_from_right() {
        let word = string("#@! andres !@#");
        let expected = string("#@! andres !@");

        let actual = action(&word, Tag::unknown(), Some('#'), &trim_right).unwrap();
        assert_eq!(actual, expected);
    }
}
