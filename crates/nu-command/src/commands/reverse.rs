use crate::prelude::*;
use nu_engine::WholeStreamCommand;
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

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        reverse(args)
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

#[allow(clippy::needless_collect)]
fn reverse(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let input = args.input.collect::<Vec<_>>();
    Ok((input.into_iter().rev().map(ReturnSuccess::value)).into_action_stream())
}

#[cfg(test)]
mod tests {
    use super::Reverse;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Reverse {})
    }
}
