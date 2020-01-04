use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::deserializer::NumericRange;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tagged;

#[derive(Deserialize)]
struct RangeArgs {
    area: Tagged<NumericRange>,
}

pub struct Range;

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

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, range)?.run()
    }
}

fn range(
    RangeArgs { area }: RangeArgs,
    RunnableContext { input, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let range = area.item;
    let (from, _) = range.from;
    let (to, _) = range.to;

    Ok(OutputStream::from_input(
        input.values.skip(*from).take(*to - *from + 1),
    ))
}
