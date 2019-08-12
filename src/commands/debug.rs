use crate::errors::ShellError;
use crate::prelude::*;

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
