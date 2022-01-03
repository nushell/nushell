use crate::prelude::*;
use itertools::Either;
use nu_engine::{FromValue, WholeStreamCommand};
use nu_errors::ShellError;
use nu_protocol::{Range, Signature, SpannedTypeName, SyntaxShape, Value};
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
                "the number of the row to drop or a range to drop consecutive rows",
            )
            .rest(
                "rest",
                SyntaxShape::Any,
                "Optionally drop more rows (Ignored if first argument is a range)",
            )
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
                result: Some(vec![Value::from("first"), Value::from("fifth")]),
            },
        ]
    }
}

fn extract_int_or_range(args: &CommandArgs) -> Result<Either<u64, Range>, ShellError> {
    let value = args.req::<Value>(0)?;

    let int_opt = value.as_u64().map(Either::Left).ok();
    let range_opt = FromValue::from_value(&value).map(Either::Right).ok();

    int_opt.or(range_opt).ok_or(ShellError::type_error(
        "int or range",
        value.spanned_type_name(),
    ))
}

fn drop(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let number_or_range = extract_int_or_range(&args)?;
    let rows = match number_or_range {
        Either::Left(row_number) => {
            let and_rows: Vec<Tagged<u64>> = args.rest(1)?;

            let mut rows: Vec<_> = and_rows.into_iter().map(|x| x.item as usize).collect();
            rows.push(row_number as usize);
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
