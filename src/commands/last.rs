use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, Value};
use nu_source::Tagged;

pub struct Last;

#[derive(Deserialize)]
pub struct LastArgs {
    rows: Option<Tagged<u64>>,
}

impl WholeStreamCommand for Last {
    fn name(&self) -> &str {
        "last"
    }

    fn signature(&self) -> Signature {
        Signature::build("last").optional(
            "rows",
            SyntaxShape::Number,
            "starting from the back, the number of rows to return",
        )
    }

    fn usage(&self) -> &str {
        "Show only the last number of rows."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, last)?.run()
    }
}

fn last(LastArgs { rows }: LastArgs, context: RunnableContext) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        let v: Vec<_> = context.input.into_vec().await;

        let rows_desired = if let Some(quantity) = rows {
            *quantity
        } else {
         1
        };

        let count = (rows_desired as usize);
        if count < v.len() {
            let k = v.len() - count;
            for x in v[k..].iter() {
                let y: Value = x.clone();
                yield ReturnSuccess::value(y)
            }
        }
    };
    Ok(stream.to_output_stream())
}
