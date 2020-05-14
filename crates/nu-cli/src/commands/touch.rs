use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tagged;
use std::fs::OpenOptions;
use std::path::PathBuf;

pub struct Touch;

#[derive(Deserialize)]
pub struct TouchArgs {
    pub target: Tagged<PathBuf>,
}

impl WholeStreamCommand for Touch {
    fn name(&self) -> &str {
        "touch"
    }
    fn signature(&self) -> Signature {
        Signature::build("touch").required(
            "filename",
            SyntaxShape::Path,
            "the path of the file you want to create",
        )
    }
    fn usage(&self) -> &str {
        "creates a file"
    }
    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        touch(args, registry)
    }
}
fn touch(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let stream = async_stream! {
        let TouchArgs { target } = args.process_raw(&registry).await?;
        match OpenOptions::new().write(true).create(true).open(&target) {
            Ok(_) => {},
            Err(err) => yield Err(ShellError::labeled_error(
                "File Error",
                err.to_string(),
                &target.tag,
            )),
        }
    };

    Ok(stream.to_output_stream())
}
