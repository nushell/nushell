use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, Value};

#[derive(Deserialize)]
struct PrependArgs {
    row: Value,
}

pub struct Prepend;

impl WholeStreamCommand for Prepend {
    fn name(&self) -> &str {
        "prepend"
    }

    fn signature(&self) -> Signature {
        Signature::build("prepend").required(
            "row value",
            SyntaxShape::Any,
            "the value of the row to prepend to the table",
        )
    }

    fn usage(&self) -> &str {
        "Prepend the given row to the front of the table"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, prepend)?.run()
    }
}

fn prepend(
    PrependArgs { row }: PrependArgs,
    RunnableContext { input, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let mut prepend: VecDeque<Value> = VecDeque::new();
    prepend.push_back(row);

    Ok(OutputStream::from_input(prepend.chain(input.values)))
}
