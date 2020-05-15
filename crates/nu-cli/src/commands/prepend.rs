use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, Value};

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
        prepend(args, registry)
    }

    fn examples(&self) -> &[Example] {
        &[Example {
            description: "Add something to the beginning of a list or table",
            example: "echo [2 3 4] | prepend 1",
        }]
    }
}

fn prepend(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();

    let stream = async_stream! {
        let (PrependArgs { row }, mut input) = args.process(&registry).await?;

        yield ReturnSuccess::value(row);
        while let Some(item) = input.next().await {
            yield ReturnSuccess::value(item);
        }
    };

    Ok(stream.to_output_stream())
}
