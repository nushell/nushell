use crate::prelude::*;
use nu_engine::WholeStreamCommand;

use nu_errors::ShellError;
use nu_parser::expand_path;
use nu_protocol::{Signature, SyntaxShape};
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
    let ctx = EvaluationContext::from_args(&args);
    let (SourceArgs { filename }, _) = args.process().await?;

    // Note: this is a special case for setting the context from a command
    // In this case, if we don't set it now, we'll lose the scope that this
    // variable should be set into.
    let contents = std::fs::read_to_string(expand_path(&filename.item).into_owned());
    match contents {
        Ok(contents) => {
            let result = crate::script::run_script_standalone(contents, true, &ctx, false).await;

            if let Err(err) = result {
                ctx.error(err.into());
            }
            Ok(OutputStream::empty())
        }
        Err(_) => {
            ctx.error(ShellError::labeled_error(
                "Can't load file to source",
                "can't load file",
                filename.span(),
            ));

            Ok(OutputStream::empty())
        }
    }
}
