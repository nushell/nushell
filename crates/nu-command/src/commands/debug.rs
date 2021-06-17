use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};

pub struct Debug;

impl WholeStreamCommand for Debug {
    fn name(&self) -> &str {
        "debug"
    }

    fn signature(&self) -> Signature {
        Signature::build("debug").switch("raw", "Prints the raw value representation.", Some('r'))
    }

    fn usage(&self) -> &str {
        "Print the Rust debug representation of the values."
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        debug_value(args)
    }
}

fn debug_value(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let raw = args.has_flag("raw");
    let input = args.input;

    Ok(input
        .map(move |v| {
            if raw {
                ReturnSuccess::value(
                    UntaggedValue::string(format!("{:#?}", v)).into_untagged_value(),
                )
            } else {
                ReturnSuccess::debug_value(v)
            }
        })
        .into_action_stream())
}

#[cfg(test)]
mod tests {
    use super::Debug;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Debug {})
    }
}
