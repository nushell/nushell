use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue};
use nu_source::Tagged;

pub struct Command;

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
        "Remove the last number of rows or columns."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        drop(args)
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

fn drop(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let rows: Option<Tagged<u64>> = args.opt(0)?;
    let v: Vec<_> = args.input.into_vec();

    let rows_to_drop = if let Some(quantity) = rows {
        *quantity as usize
    } else {
        1
    };

    Ok(if rows_to_drop == 0 {
        v.into_iter().map(Ok).into_input_stream()
    } else {
        let k = if v.len() < rows_to_drop {
            0
        } else {
            v.len() - rows_to_drop
        };

        let iter = v.into_iter().map(Ok).take(k);

        iter.into_input_stream()
    })
}
