use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{CallInfo, Signature, SyntaxShape, Value};
use nu_source::Tagged;
use std::fs::OpenOptions;
use std::path::PathBuf;

pub struct Touch;

#[derive(Deserialize)]
pub struct TouchArgs {
    pub target: Tagged<PathBuf>,
}

impl PerItemCommand for Touch {
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
        call_info: &CallInfo,
        _registry: &CommandRegistry,
        raw_args: &RawCommandArgs,
        _input: Value,
    ) -> Result<OutputStream, ShellError> {
        call_info
            .process(&raw_args.shell_manager, raw_args.ctrl_c.clone(), touch)?
            .run()
    }
}
fn touch(args: TouchArgs, _context: &RunnablePerItemContext) -> Result<OutputStream, ShellError> {
    match OpenOptions::new()
        .write(true)
        .create(true)
        .open(&args.target)
    {
        Ok(_) => Ok(OutputStream::empty()),
        Err(err) => Err(ShellError::labeled_error(
            "File Error",
            err.to_string(),
            &args.target.tag,
        )),
    }
}
