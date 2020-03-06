use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::Signature;

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
}

fn reverse(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let (input, _args) = args.parts();

    let input = input.values.collect::<Vec<_>>();

    let output = input.map(move |mut vec| {
        vec.reverse();
        futures::stream::iter(vec)
    });

    Ok(output.flatten_stream().from_input_stream())
}
