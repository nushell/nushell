use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};

pub struct Debug;

#[derive(Deserialize)]
pub struct DebugArgs {
    raw: bool,
}

impl WholeStreamCommand for Debug {
    fn name(&self) -> &str {
        "debug"
    }

    fn signature(&self) -> Signature {
        Signature::build("debug").switch("raw", "Prints the raw value representation.", Some('r'))
    }

    fn usage(&self) -> &str {
        "Print the Rust debug representation of the values"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, debug_value)?.run()
    }
}

fn debug_value(
    DebugArgs { raw }: DebugArgs,
    RunnableContext { input, .. }: RunnableContext,
) -> Result<impl ToOutputStream, ShellError> {
    Ok(input
        .values
        .map(move |v| {
            if raw {
                ReturnSuccess::value(
                    UntaggedValue::string(format!("{:#?}", v)).into_untagged_value(),
                )
            } else {
                ReturnSuccess::debug_value(v)
            }
        })
        .to_output_stream())
}
