use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue, Value};

struct RangeArgs {
    range: nu_protocol::Range,
}

pub struct Range;

impl WholeStreamCommand for Range {
    fn name(&self) -> &str {
        "range"
    }

    fn signature(&self) -> Signature {
        Signature::build("range").required(
            "rows",
            SyntaxShape::Range,
            "range of rows to return: Eg) 4..7 (=> from 4 to 7)",
        )
    }

    fn usage(&self) -> &str {
        "Return only the selected rows."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Return rows 1 through 3",
                example: "echo [1 2 3 4 5] | range 1..3",
                result: Some(vec![
                    UntaggedValue::int(2).into(),
                    UntaggedValue::int(3).into(),
                    UntaggedValue::int(4).into(),
                ]),
            },
            Example {
                description: "Return the third row from the end, through to the end",
                example: "echo [1 2 3 4 5] | range (-3..)",
                result: Some(vec![
                    UntaggedValue::int(3).into(),
                    UntaggedValue::int(4).into(),
                    UntaggedValue::int(5).into(),
                ]),
            },
        ]
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        range(args)
    }
}

fn range(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let cmd_args = RangeArgs {
        range: args.req(0)?,
    };

    let from_raw = cmd_args.range.min_i64()?;
    let to_raw = cmd_args.range.max_i64()?;
    // only collect the input if we have any negative indices
    if from_raw < 0 || to_raw < 0 {
        let input = args.input.into_vec();
        let input_size = input.len() as i64;

        let from = if from_raw < 0 {
            (input_size + from_raw) as usize
        } else {
            from_raw as usize
        };

        let to = if to_raw < 0 {
            (input_size + to_raw) as usize
        } else if to_raw > input.len() as i64 {
            input.len()
        } else {
            to_raw as usize
        };

        if from > to {
            Ok(OutputStream::one(Value::nothing()))
        } else {
            Ok(OutputStream::from(input[from..to].to_vec()))
        }
    } else {
        let from = from_raw as usize;
        let to = to_raw as usize;
        if from > to {
            Ok(OutputStream::one(Value::nothing()))
        } else {
            Ok(args
                .input
                .skip(from)
                .take(to - from + 1)
                .into_output_stream())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Range;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Range {})
    }
}
