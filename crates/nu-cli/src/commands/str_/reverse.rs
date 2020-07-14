use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str reverse"
    }

    fn signature(&self) -> Signature {
        Signature::build("str reverse")
    }

    fn usage(&self) -> &str {
        "outputs the reversals of the strings in the pipeline"
    }

    async fn run(
        &self,
        args: CommandArgs,
        _registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        Ok(args
            .input
            .map(move |x| match x.as_string() {
                Ok(s) => ReturnSuccess::value(
                    UntaggedValue::string(s.chars().rev().collect::<String>())
                        .into_untagged_value(),
                ),
                Err(err) => Err(err),
            })
            .to_output_stream())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Return the reversals of multiple strings",
            example: "echo 'Nushell' | str reverse",
            result: Some(vec![UntaggedValue::string("llehsuN").into_untagged_value()]),
        }]
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
