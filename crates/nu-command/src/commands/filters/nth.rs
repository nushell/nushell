use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, Value};
use nu_source::Tagged;

pub struct Nth;

impl WholeStreamCommand for Nth {
    fn name(&self) -> &str {
        "nth"
    }

    fn signature(&self) -> Signature {
        Signature::build("nth")
            .required(
                "row number",
                SyntaxShape::Int,
                "the number of the row to return",
            )
            .rest("rest", SyntaxShape::Any, "Optionally return more rows")
            .switch("skip", "Skip the rows instead of selecting them", Some('s'))
    }

    fn usage(&self) -> &str {
        "Return or skip only the selected rows."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        nth(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get the second row",
                example: "echo [first second third] | nth 1",
                result: Some(vec![Value::from("second")]),
            },
            Example {
                description: "Get the first and third rows",
                example: "echo [first second third] | nth 0 2",
                result: Some(vec![Value::from("first"), Value::from("third")]),
            },
            Example {
                description: "Skip the first and third rows",
                example: "echo [first second third] | nth --skip 0 2",
                result: Some(vec![Value::from("second")]),
            },
        ]
    }
}

fn nth(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let row_number: Tagged<u64> = args.req(0)?;
    let and_rows: Vec<Tagged<u64>> = args.rest(1)?;
    let skip = args.has_flag("skip");
    let input = args.input;

    let mut rows: Vec<_> = and_rows.into_iter().map(|x| x.item as usize).collect();
    rows.push(row_number.item as usize);
    rows.sort_unstable();

    Ok(NthIterator {
        input,
        rows,
        skip,
        current: 0,
    }
    .into_output_stream())
}

struct NthIterator {
    input: InputStream,
    rows: Vec<usize>,
    skip: bool,
    current: usize,
}

impl Iterator for NthIterator {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if !self.skip {
                if let Some(row) = self.rows.get(0) {
                    if self.current == *row {
                        self.rows.remove(0);
                        self.current += 1;
                        return self.input.next();
                    } else {
                        self.current += 1;
                        let _ = self.input.next();
                        continue;
                    }
                } else {
                    return None;
                }
            } else if let Some(row) = self.rows.get(0) {
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
