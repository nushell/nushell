use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape};
use nu_source::Tagged;

pub struct Skip;

#[derive(Deserialize)]
pub struct SkipArgs {
    rows: Option<Tagged<usize>>,
}

impl WholeStreamCommand for Skip {
    fn name(&self) -> &str {
        "skip"
    }

    fn signature(&self) -> Signature {
        Signature::build("skip").optional("rows", SyntaxShape::Int, "how many rows to skip")
    }

    fn usage(&self) -> &str {
        "Skip some number of rows."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        skip(args, registry)
    }

    fn examples(&self) -> &[Example] {
        &[Example {
            description: "Skip the first 5 rows",
            example: "ls | skip 5",
        }]
    }
}

fn skip(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let stream = async_stream! {
        let (SkipArgs { rows }, mut input) = args.process(&registry).await?;
        let mut rows_desired = if let Some(quantity) = rows {
            *quantity
        } else {
            1
        };

        for input in input.next().await {
            if rows_desired > 0{
                rows_desired -= 1;
            }

            if rows_desired == 0 {
                yield ReturnSuccess::value(input);
            }
        }
    };

    Ok(stream.to_output_stream())
}
