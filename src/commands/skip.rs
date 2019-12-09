use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tagged;

pub struct Skip;

#[derive(Deserialize)]
pub struct SkipArgs {
    rows: Option<Tagged<u64>>,
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
        args.process(registry, skip)?.run()
    }
}

fn skip(SkipArgs { rows }: SkipArgs, context: RunnableContext) -> Result<OutputStream, ShellError> {
    let rows_desired = if let Some(quantity) = rows {
        *quantity
    } else {
        1
    };

    Ok(OutputStream::from_input(
        context.input.values.skip(rows_desired),
    ))
}
