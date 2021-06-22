use crate::prelude::*;
use nu_engine::{script, WholeStreamCommand};

use nu_errors::ShellError;
use nu_path::expand_path;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tagged;

use std::{borrow::Cow, path::Path};

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
            SyntaxShape::FilePath,
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
    let ctx = &args.context;
    let filename: Tagged<String> = args.req(0)?;

    // Note: this is a special case for setting the context from a command
    // In this case, if we don't set it now, we'll lose the scope that this
    // variable should be set into.
    let contents = std::fs::read_to_string(&expand_path(Cow::Borrowed(Path::new(&filename.item))));
    match contents {
        Ok(contents) => {
            let result = script::run_script_standalone(contents, true, &ctx, false);

            if let Err(err) = result {
                ctx.error(err);
            }
            Ok(ActionStream::empty())
        }
        Err(_) => {
            ctx.error(ShellError::labeled_error(
                "Can't load file to source",
                "can't load file",
                filename.span(),
            ));

            Ok(ActionStream::empty())
        }
    }
}
