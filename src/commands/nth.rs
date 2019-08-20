use crate::commands::WholeStreamCommand;
use crate::errors::ShellError;
use crate::parser::CommandRegistry;
use crate::prelude::*;

#[derive(Deserialize)]
struct NthArgs {
    position: Tagged<i64>,
}

pub struct Nth;

impl WholeStreamCommand for Nth {
    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, nth)?.run()
    }

    fn name(&self) -> &str {
        "nth"
    }

    fn signature(&self) -> Signature {
        Signature::build("nth").optional("amount", SyntaxType::Any)
    }
}

fn nth(
    NthArgs { position: amount }: NthArgs,
    RunnableContext { input, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    Ok(OutputStream::from_input(
        input.values.skip(amount.item as u64).take(1),
    ))
}
