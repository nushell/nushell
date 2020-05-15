use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape};
use nu_source::Tagged;

pub struct First;

#[derive(Deserialize)]
pub struct FirstArgs {
    rows: Option<Tagged<usize>>,
}

impl WholeStreamCommand for First {
    fn name(&self) -> &str {
        "first"
    }

    fn signature(&self) -> Signature {
        Signature::build("first").optional(
            "rows",
            SyntaxShape::Int,
            "starting from the front, the number of rows to return",
        )
    }

    fn usage(&self) -> &str {
        "Show only the first number of rows."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        first(args, registry)
    }

    fn examples(&self) -> &[Example] {
        &[
            Example {
                description: "Return the first item of a list/table",
                example: "echo [1 2 3] | first",
            },
            Example {
                description: "Return the first 2 items of a list/table",
                example: "echo [1 2 3] | first 2",
            },
        ]
    }
}

fn first(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let stream = async_stream! {
        let (FirstArgs { rows }, mut input) = args.process(&registry).await?;
        let mut rows_desired = if let Some(quantity) = rows {
            *quantity
        } else {
            1
        };

        while let Some(input) = input.next().await {
            if rows_desired > 0 {
                yield ReturnSuccess::value(input);
            } else {
                break;
            }

            if rows_desired > 0 {
                rows_desired -= 1;
            }
        }
    };

    Ok(stream.to_output_stream())
}
