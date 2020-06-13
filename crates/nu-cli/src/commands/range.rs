use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::deserializer::NumericRange;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape};
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

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        range(args, registry).await
    }
}

async fn range(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let (RangeArgs { area }, input) = args.process(&registry).await?;
    let range = area.item;
    let (from, _) = range.from;
    let (to, _) = range.to;

    let from = *from as usize;
    let to = *to as usize;

    Ok(input
        .skip(from)
        .take(to - from + 1)
        .map(ReturnSuccess::value)
        .to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Range;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Range {})
    }
}
