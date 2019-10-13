use crate::commands::WholeStreamCommand;
use crate::data::{Primitive, TaggedDictBuilder, Value};
use crate::prelude::*;

pub struct FromSSV;

#[derive(Deserialize)]
pub struct FromSSVArgs {
    headerless: bool,
}

const STRING_REPRESENTATION: &str = "from-ssv";

impl WholeStreamCommand for FromSSV {
    fn name(&self) -> &str {
        STRING_REPRESENTATION
    }

    fn signature(&self) -> Signature {
        Signature::build(STRING_REPRESENTATION).switch("headerless")
    }

    fn usage(&self) -> &str {
        "Parse text as .ssv and create a table."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, from_ssv)?.run()
    }
}

fn from_ssv(
    FromSSVArgs {
        headerless: headerless,
    }: FromSSVArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    unimplemented!()
}
