use crate::commands::WholeStreamCommand;
use crate::errors::ShellError;
use crate::parser::CommandRegistry;
use crate::prelude::*;

pub struct First;

#[derive(Deserialize)]
pub struct FirstArgs {
    amount: Tagged<u64>,
}

impl WholeStreamCommand for First {
    fn name(&self) -> &str {
        "first"
    }

    fn signature(&self) -> Signature {
        Signature::build("first").required("amount", SyntaxShape::Int)
    }

    fn usage(&self) -> &str {
        "Show only the first number of rows."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, first)?.run()
    }
}

fn first(
    FirstArgs { amount }: FirstArgs,
    context: RunnableContext,
) -> Result<OutputStream, ShellError> {
    Ok(OutputStream::from_input(context.input.values.take(*amount)))
}
