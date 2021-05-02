use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{RangeInclusion, Signature, SyntaxShape, Value};

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

    let (from, left_inclusive) = cmd_args.range.from;
    let (to, right_inclusive) = cmd_args.range.to;
    let from_span = from.span;
    let to_span = to.span;

    let from = from
        .map(|from| from.as_usize(from_span))
        .item
        .unwrap_or(0)
        .saturating_add(if left_inclusive == RangeInclusion::Inclusive {
            0
        } else {
            1
        });

    let to = to
        .map(|to| to.as_usize(to_span))
        .item
        .unwrap_or(usize::MAX)
        .saturating_sub(if right_inclusive == RangeInclusion::Inclusive {
            0
        } else {
            1
        });

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
