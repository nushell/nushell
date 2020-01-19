use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};

pub struct Debug;

#[derive(Deserialize)]
pub struct DebugArgs {}

impl WholeStreamCommand for Debug {
    fn name(&self) -> &str {
        "debug"
    }

    fn signature(&self) -> Signature {
        Signature::build("debug")
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
    _args: DebugArgs,
    RunnableContext { input, .. }: RunnableContext,
) -> Result<impl ToOutputStream, ShellError> {
    Ok(input
        .values
        .map(|v| {
            ReturnSuccess::value(UntaggedValue::string(format!("{:#?}", v)).into_untagged_value())
        })
        .to_output_stream())
}
