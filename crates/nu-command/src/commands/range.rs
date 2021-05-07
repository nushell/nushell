use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, Value};

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

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        range(args)
    }
}

fn range(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once()?;
    let cmd_args = RangeArgs {
        range: args.req(0)?,
    };

    let from = cmd_args.range.min_usize()?;
    let to = cmd_args.range.max_usize()?;

    if from > to {
        Ok(OutputStream::one(Value::nothing()))
    } else {
        Ok(args.input.skip(from).take(to - from + 1).to_output_stream())
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
