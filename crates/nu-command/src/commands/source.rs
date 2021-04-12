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

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        source(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }
}

pub fn source(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let ctx = EvaluationContext::from_args(&args);
    let (SourceArgs { filename }, _) = args.process()?;

    let script = NuScript::File(PathBuf::from(expand_path(&filename.item).to_string()));
    let options = RunScriptOptions::default()
        .source_script(true)
        .redirect_stdin(true)
        .exit_on_error(false);
    script::run_script(script, &options, &ctx);

    Ok(ActionStream::empty())
}
