use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str length"
    }

    fn signature(&self) -> Signature {
        Signature::build("str length")
    }

    fn usage(&self) -> &str {
        "outputs the lengths of the strings in the pipeline"
    }

    async fn run(
        &self,
        args: CommandArgs,
        _registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        Ok(args
            .input
            .map(move |x| match x.as_string() {
                Ok(s) => ReturnSuccess::value(UntaggedValue::int(s.len()).into_untagged_value()),
                Err(err) => Err(err),
            })
            .to_output_stream())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Return the lengths of multiple strings",
                example: "echo 'hello' | str length",
                result: Some(vec![UntaggedValue::int(5).into_untagged_value()]),
            },
            Example {
                description: "Return the lengths of multiple strings",
                example: "echo 'hi' 'there' | str length",
                result: Some(vec![
                    UntaggedValue::int(2).into_untagged_value(),
                    UntaggedValue::int(5).into_untagged_value(),
                ]),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::SubCommand;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }
}
