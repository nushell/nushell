use crate::prelude::*;
use nu_engine::deserializer::NumericRange;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{RangeInclusion, ReturnSuccess, Signature, SyntaxShape};
use nu_source::Tagged;

#[derive(Deserialize)]
struct RangeArgs {
    area: Tagged<NumericRange>,
}

pub struct Range;

#[async_trait]
impl WholeStreamCommand for Range {
    fn name(&self) -> &str {
        "range"
    }

    fn signature(&self) -> Signature {
        Signature::build("range").required(
            "rows ",
            SyntaxShape::Range,
            "range of rows to return: Eg) 4..7 (=> from 4 to 7)",
        )
    }

    fn usage(&self) -> &str {
        "Return only the selected rows"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        range(args).await
    }
}

async fn range(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let (RangeArgs { area }, input) = args.process().await?;
    let range = area.item;
    let (from, left_inclusive) = range.from;
    let (to, right_inclusive) = range.to;
    let from = from.map(|from| *from as usize).unwrap_or(0).saturating_add(
        if left_inclusive == RangeInclusion::Inclusive {
            0
        } else {
            1
        },
    );
    let to = to
        .map(|to| *to as usize)
        .unwrap_or(usize::MAX)
        .saturating_sub(if right_inclusive == RangeInclusion::Inclusive {
            0
        } else {
            1
        });

    Ok(input
        .skip(from)
        .take(to - from + 1)
        .map(ReturnSuccess::value)
        .to_output_stream())
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
