use crate::commands::WholeStreamCommand;
use crate::prelude::*;

use nu_errors::ShellError;
use nu_protocol::{CommandAction, ReturnSuccess, Signature, SyntaxShape};
use nu_source::Tagged;

pub struct Source;

#[derive(Deserialize)]
pub struct SourceArgs {
    pub filename: Tagged<String>,
}

#[async_trait]
impl WholeStreamCommand for Source {
    fn name(&self) -> &str {
        "source"
    }

    fn signature(&self) -> Signature {
        Signature::build("source").required(
            "filename",
            SyntaxShape::String,
            "the filepath to the script file to source",
        )
    }

    fn usage(&self) -> &str {
        "Runs a script file in the current context."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        source(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }
}

pub async fn source(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let (SourceArgs { filename }, _) = args.process().await?;

    Ok(OutputStream::one(ReturnSuccess::action(
        CommandAction::SourceScript(filename.item),
    )))
}
