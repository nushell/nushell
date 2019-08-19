use crate::commands::WholeStreamCommand;
use crate::errors::ShellError;
use crate::prelude::*;

pub struct Debug;

impl WholeStreamCommand for Debug {
    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        debug(args, registry)
    }

    fn name(&self) -> &str {
        "debug"
    }

    fn signature(&self) -> Signature {
        // TODO: Signature?
        Signature::build("debug")
    }
}

pub fn debug(args: CommandArgs, _registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let input = args.input;

    Ok(input
        .values
        .map(|v| {
            println!("{:?}", v);
            ReturnSuccess::value(v)
        })
        .to_output_stream())
}
