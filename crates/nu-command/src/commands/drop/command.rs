use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue};
use nu_source::Tagged;

pub struct Command;

#[derive(Deserialize)]
pub struct Arguments {
    rows: Option<Tagged<u64>>,
}

#[async_trait]
impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "drop"
    }

    fn signature(&self) -> Signature {
        Signature::build("drop").optional(
            "rows",
            SyntaxShape::Number,
            "starting from the back, the number of rows to remove",
        )
    }

    fn usage(&self) -> &str {
        "Remove the last number of rows. If you want to remove columns, try 'drop column'."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        drop(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Remove the last item of a list/table",
                example: "echo [1 2 3] | drop",
                result: Some(vec![
                    UntaggedValue::int(1).into(),
                    UntaggedValue::int(2).into(),
                ]),
            },
            Example {
                description: "Remove the last 2 items of a list/table",
                example: "echo [1 2 3] | drop 2",
                result: Some(vec![UntaggedValue::int(1).into()]),
            },
        ]
    }
}

async fn drop(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let (Arguments { rows }, input) = args.process().await?;
    let v: Vec<_> = input.into_vec().await;

    let rows_to_drop = if let Some(quantity) = rows {
        *quantity as usize
    } else {
        1
    };

    Ok(if rows_to_drop == 0 {
        futures::stream::iter(v).to_output_stream()
    } else {
        let k = if v.len() < rows_to_drop {
            0
        } else {
            v.len() - rows_to_drop
        };

        let iter = v.into_iter().take(k);

        futures::stream::iter(iter).to_output_stream()
    })
}
