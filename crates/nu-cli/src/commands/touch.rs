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
        args.process(registry, touch)?.run()
    }
}
fn touch(TouchArgs { target }: TouchArgs, _: RunnableContext) -> Result<OutputStream, ShellError> {
    match OpenOptions::new().write(true).create(true).open(&target) {
        Ok(_) => Ok(OutputStream::empty()),
        Err(err) => Err(ShellError::labeled_error(
            "File Error",
            err.to_string(),
            &target.tag,
        )),
    }
}
