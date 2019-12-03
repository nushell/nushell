use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, Value};

#[derive(Deserialize)]
struct AppendArgs {
    row: Value,
}

pub struct Append;

impl WholeStreamCommand for Append {
    fn name(&self) -> &str {
        "append"
    }

    fn signature(&self) -> Signature {
        Signature::build("append").required(
            "row value",
            SyntaxShape::Any,
            "the value of the row to append to the table",
        )
    }

    fn usage(&self) -> &str {
        "Append the given row to the table"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, append)?.run()
    }
}

fn append(
    AppendArgs { row }: AppendArgs,
    RunnableContext { input, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let mut after: VecDeque<Value> = VecDeque::new();
    after.push_back(row);

    Ok(OutputStream::from_input(input.values.chain(after)))
}
