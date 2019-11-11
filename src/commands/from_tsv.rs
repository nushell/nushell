use crate::commands::from_structured_data::from_structured_data;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;

pub struct FromTSV;

#[derive(Deserialize)]
pub struct FromTSVArgs {
    headerless: bool,
}

impl WholeStreamCommand for FromTSV {
    fn name(&self) -> &str {
        "from-tsv"
    }

    fn signature(&self) -> Signature {
        Signature::build("from-tsv")
            .switch("headerless", "don't treat the first row as column names")
    }

    fn usage(&self) -> &str {
        "Parse text as .tsv and create table."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, from_tsv)?.run()
    }
}

fn from_tsv(
    FromTSVArgs { headerless }: FromTSVArgs,
    runnable_context: RunnableContext,
) -> Result<OutputStream, ShellError> {
    from_structured_data(headerless, '\t', "TSV", runnable_context)
}
