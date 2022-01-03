use crate::prelude::*;
use itertools::Either;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, Value, Range, SpannedTypeName};
use nu_source::Tagged;

pub struct SubCommand;

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "drop nth"
    }

    fn signature(&self) -> Signature {
        Signature::build("drop nth")
            .required(
                "row number or row range",
                // FIXME: we can make this accept either Int or Range when we can compose SyntaxShapes
                SyntaxShape::Any,
                "the number of the row to drop",
            )
            .rest("rest", SyntaxShape::Any, "Optionally drop more rows (Only if first argument is number)")
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
            Example {
                description: "Drop range rows from second to fourth",
                example: "echo [first second third fourth fifth] | drop nth (1..3)",
                result: Some(vec![Value::from("first fifth")]),
            }
        ]
    }
}

fn extract_int_or_range(args: &CommandArgs) -> Result<Either<Tagged<u64>, Tagged<Range>>, ShellError> {
    let actual_type = args.req::<Value>(0).map(|value| value.spanned_type_name())?;
    match args.req::<Tagged<u64>>(0).ok() {
        Some(row_number) => Some(Either::Left(row_number)),
        None => args.req::<Tagged<Range>>(0).map(Either::Right).ok(),
    }.ok_or(ShellError::type_error("int or range", actual_type))
}

fn drop(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let number_or_range = extract_int_or_range(&args)?;
    let rows = match number_or_range {
        Either::Left(row_number) => {
            let and_rows: Vec<Tagged<u64>> = args.rest(1)?;

            let mut rows: Vec<_> = and_rows.into_iter().map(|x| x.item as usize).collect();
            rows.push(row_number.item as usize);
            rows.sort_unstable();
            rows
        }
        Either::Right(row_range) => {
            let from = row_range.min_u64()? as usize;    
            let to = row_range.max_u64()? as usize;
            
            (from..=to).collect()
        }
    };    

    let input = args.input;

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
