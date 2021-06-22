use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue};

pub struct Last;

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

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        last(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get the last row",
                example: "echo [1 2 3] | last",
                result: Some(vec![UntaggedValue::int(3).into()]),
            },
            Example {
                description: "Get the last three rows",
                example: "echo [1 2 3 4 5] | last 3",
                result: Some(vec![
                    UntaggedValue::int(3).into(),
                    UntaggedValue::int(4).into(),
                    UntaggedValue::int(5).into(),
                ]),
            },
        ]
    }
}

fn last(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let rows: Option<usize> = args.opt(0)?;
    let v: Vec<_> = args.input.into_vec();

    let end_rows_desired = if let Some(quantity) = rows {
        quantity
    } else {
        1
    };

    let beginning_rows_to_skip = if end_rows_desired < v.len() {
        v.len() - end_rows_desired
    } else {
        0
    };

    let iter = v.into_iter().skip(beginning_rows_to_skip);

    Ok(OutputStream::from_stream(iter))
}

#[cfg(test)]
mod tests {
    use super::Last;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Last {})
    }
}
