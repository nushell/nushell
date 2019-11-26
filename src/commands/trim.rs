use crate::commands::WholeStreamCommand;

use crate::prelude::*;
use nu_protocol::{ReturnSuccess, Signature};
use nu_errors::ShellError;

pub struct Trim;

impl WholeStreamCommand for Trim {
    fn name(&self) -> &str {
        "trim"
    }

    fn signature(&self) -> Signature {
        Signature::build("trim")
    }

    fn usage(&self) -> &str {
        "Trim leading and following whitespace from text data."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        trim(args, registry)
    }
}

fn trim(args: CommandArgs, _registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let input = args.input;

    Ok(input
        .values
        .map(move |v| {
            let string = String::extract(&v)?;
            ReturnSuccess::value(value::string(string.trim()).into_value(v.tag()))
        })
        .to_output_stream())
}
