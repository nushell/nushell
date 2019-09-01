use crate::commands::WholeStreamCommand;
use crate::errors::ShellError;
use crate::parser::CommandRegistry;
use crate::prelude::*;

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

    let output = input.values.collect::<Vec<_>>();

    let output = output.map(move |mut vec| {
        vec.reverse();
        vec.into_iter().collect::<VecDeque<_>>()
    });

    Ok(output.flatten_stream().from_input_stream())
}
