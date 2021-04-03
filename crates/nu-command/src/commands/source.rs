use std::path::PathBuf;

use crate::prelude::*;
use nu_engine::{script, WholeStreamCommand};

use nu_errors::ShellError;
use nu_parser::expand_path;
use nu_protocol::{NuScript, RunScriptOptions, Signature, SyntaxShape};
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

    let script = NuScript::File(PathBuf::from(expand_path(&filename.item).to_string()));
    let options = RunScriptOptions::default()
        .use_existing_scope(true)
        .redirect_stdin(true)
        .exit_on_error(false);
    script::run_script(script, &options, &ctx).await;

    Ok(OutputStream::empty())
}
