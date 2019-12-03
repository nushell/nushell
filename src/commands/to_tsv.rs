use crate::commands::to_delimited_data::to_delimited_data;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::Signature;

pub struct ToTSV;

#[derive(Deserialize)]
pub struct ToTSVArgs {
    headerless: bool,
}

impl WholeStreamCommand for ToTSV {
    fn name(&self) -> &str {
        "to-tsv"
    }

    fn signature(&self) -> Signature {
        Signature::build("to-tsv").switch(
            "headerless",
            "do not output the column names as the first row",
        )
    }

    fn usage(&self) -> &str {
        "Convert table into .tsv text"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, to_tsv)?.run()
    }
}

fn to_tsv(
    ToTSVArgs { headerless }: ToTSVArgs,
    runnable_context: RunnableContext,
) -> Result<OutputStream, ShellError> {
    to_delimited_data(headerless, '\t', "TSV", runnable_context)
}
