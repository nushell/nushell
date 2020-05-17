use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};

pub struct Reverse;

impl WholeStreamCommand for Reverse {
    fn name(&self) -> &str {
        "reverse"
    }

    fn signature(&self) -> Signature {
        Signature::build("reverse")
    }

    fn usage(&self) -> &str {
        "Reverses the table."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        reverse(args, registry)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Sort list of numbers in descending file size",
            example: "echo [3 1 2 19 0] | reverse",
            result: Some(vec![
                UntaggedValue::int(0).into(),
                UntaggedValue::int(19).into(),
                UntaggedValue::int(2).into(),
                UntaggedValue::int(1).into(),
                UntaggedValue::int(3).into(),
            ]),
        }]
    }
}

fn reverse(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let stream = async_stream! {
        let args = args.evaluate_once(&registry).await?;
        let (input, _args) = args.parts();

        let input = input.collect::<Vec<_>>().await;
        for output in input.into_iter().rev() {
            yield ReturnSuccess::value(output);
        }
    };

    Ok(stream.to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Reverse;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Reverse {})
    }
}
