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

    fn examples(&self) -> &[Example] {
        &[Example {
            description: "Add something to the beginning of a list or table",
            example: "echo [2 3 4] | prepend 1",
        }]
    }
}

fn prepend(
    PrependArgs { row }: PrependArgs,
    RunnableContext { input, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let prepend = futures::stream::iter(vec![row]);

    Ok(prepend.chain(input).to_output_stream())
}
