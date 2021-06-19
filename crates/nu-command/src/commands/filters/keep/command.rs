use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue};
use nu_source::Tagged;

pub struct Command;

impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "keep"
    }

    fn signature(&self) -> Signature {
        Signature::build("keep").optional(
            "rows",
            SyntaxShape::Int,
            "Starting from the front, the number of rows to keep",
        )
    }

    fn usage(&self) -> &str {
        "Keep the number of rows only."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        keep(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Keep the first row",
                example: "echo [1 2 3] | keep",
                result: Some(vec![UntaggedValue::int(1).into()]),
            },
            Example {
                description: "Keep the first four rows",
                example: "echo [1 2 3 4 5] | keep 4",
                result: Some(vec![
                    UntaggedValue::int(1).into(),
                    UntaggedValue::int(2).into(),
                    UntaggedValue::int(3).into(),
                    UntaggedValue::int(4).into(),
                ]),
            },
        ]
    }
}

fn keep(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let rows: Option<Tagged<usize>> = args.opt(0)?;

    let rows_desired = if let Some(quantity) = rows {
        *quantity
    } else {
        1
    };

    Ok(args.input.take(rows_desired).into_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Command;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Command {})
    }
}
