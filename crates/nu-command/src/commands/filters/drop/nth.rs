use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, Value};
use nu_source::Tagged;

pub struct SubCommand;

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "drop nth"
    }

    fn signature(&self) -> Signature {
        Signature::build("drop nth")
            .required(
                "row number",
                SyntaxShape::Int,
                "the number of the row to drop",
            )
            .rest("rest", SyntaxShape::Any, "Optionally drop more rows")
    }

    fn usage(&self) -> &str {
        "Drops the selected rows."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        drop(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Drop the second row",
                example: "echo [first second third] | drop nth 1",
                result: Some(vec![Value::from("first"), Value::from("third")]),
            },
            Example {
                description: "Drop the first and third rows",
                example: "echo [first second third] | drop nth 0 2",
                result: Some(vec![Value::from("second")]),
            },
        ]
    }
}

fn drop(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let row_number: Tagged<u64> = args.req(0)?;
    let and_rows: Vec<Tagged<u64>> = args.rest(1)?;
    let input = args.input;

    let mut rows: Vec<_> = and_rows.into_iter().map(|x| x.item as usize).collect();
    rows.push(row_number.item as usize);
    rows.sort_unstable();

    Ok(DropNthIterator {
        input,
        rows,
        current: 0,
    }
    .into_output_stream())
}

struct DropNthIterator {
    input: InputStream,
    rows: Vec<usize>,
    current: usize,
}

impl Iterator for DropNthIterator {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(row) = self.rows.get(0) {
                if self.current == *row {
                    self.rows.remove(0);
                    self.current += 1;
                    let _ = self.input.next();
                    continue;
                } else {
                    self.current += 1;
                    return self.input.next();
                }
            } else {
                return self.input.next();
            }
        }
    }
}
