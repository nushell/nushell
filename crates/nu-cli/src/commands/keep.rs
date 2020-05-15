use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape};
use nu_source::Tagged;

pub struct Keep;

#[derive(Deserialize)]
pub struct KeepArgs {
    rows: Option<Tagged<usize>>,
}

impl WholeStreamCommand for Keep {
    fn name(&self) -> &str {
        "keep"
    }

    fn signature(&self) -> Signature {
        Signature::build("keep").optional(
            "rows",
            SyntaxShape::Int,
            "starting from the front, the number of rows to keep",
        )
    }

    fn usage(&self) -> &str {
        "Keep the number of rows only"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        keep(args, registry)
    }

    fn examples(&self) -> &[Example] {
        &[
            Example {
                description: "Keep the first row",
                example: "ls | keep",
            },
            Example {
                description: "Keep the first four rows",
                example: "ls | keep 4",
            },
        ]
    }
}

fn keep(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let stream = async_stream! {
        let (KeepArgs { rows }, mut input) = args.process(&registry).await?;
        let mut rows_desired = if let Some(quantity) = rows {
            *quantity
        } else {
            1
        };

        for input in input.next().await {
            if rows_desired > 0 {
                yield ReturnSuccess::value(input);
            } else {
                break;
            }

            if rows_desired > 0{
                rows_desired -= 1;
            }
        }
    };

    Ok(stream.to_output_stream())
}
