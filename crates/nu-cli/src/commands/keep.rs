use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
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
        args.process(registry, keep)?.run()
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

fn keep(KeepArgs { rows }: KeepArgs, context: RunnableContext) -> Result<OutputStream, ShellError> {
    let rows_desired = if let Some(quantity) = rows {
        *quantity
    } else {
        1
    };

    Ok(OutputStream::from_input(context.input.take(rows_desired)))
}
