use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, Value};
use nu_source::Tagged;

pub struct Drop;

#[derive(Deserialize)]
pub struct DropArgs {
    rows: Option<Tagged<u64>>,
}

impl WholeStreamCommand for Drop {
    fn name(&self) -> &str {
        "drop"
    }

    fn signature(&self) -> Signature {
        Signature::build("drop").optional(
            "rows",
            SyntaxShape::Number,
            "starting from the back, the number of rows to drop",
        )
    }

    fn usage(&self) -> &str {
        "Drop the last number of rows."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, drop)?.run()
    }

    fn examples(&self) -> &[Example] {
        &[
            Example {
                description: "Remove the last item of a list/table",
                example: "echo [1 2 3] | drop",
            },
            Example {
                description: "Remove the last 2 items of a list/table",
                example: "echo [1 2 3] | drop 2",
            },
        ]
    }
}

fn drop(DropArgs { rows }: DropArgs, context: RunnableContext) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        let v: Vec<_> = context.input.into_vec().await;

        let rows_to_drop = if let Some(quantity) = rows {
            *quantity as usize
        } else {
            1
        };

        if rows_to_drop < v.len() {
            let k = v.len() - rows_to_drop;
            for x in v[0..k].iter() {
                let y: Value = x.clone();
                yield ReturnSuccess::value(y)
            }
        }
    };
    Ok(stream.to_output_stream())
}
