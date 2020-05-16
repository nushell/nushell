use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, Value};

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
        append(args, registry)
    }

    fn examples(&self) -> &[Example] {
        &[Example {
            description: "Add something to the end of a list or table",
            example: "echo [1 2 3] | append 4",
        }]
    }
}

fn append(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();

    let stream = async_stream! {
        let (AppendArgs { row }, mut input) = args.process(&registry).await?;

        while let Some(item) = input.next().await {
            yield ReturnSuccess::value(item);
        }
        yield ReturnSuccess::value(row);
    };

    Ok(stream.to_output_stream())
}
