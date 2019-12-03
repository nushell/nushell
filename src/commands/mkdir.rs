use crate::commands::command::RunnablePerItemContext;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{CallInfo, Signature, SyntaxShape, Value};
use nu_source::Tagged;
use std::path::PathBuf;

pub struct Mkdir;

#[derive(Deserialize)]
pub struct MkdirArgs {
    pub rest: Vec<Tagged<PathBuf>>,
}

impl PerItemCommand for Mkdir {
    fn name(&self) -> &str {
        "mkdir"
    }

    fn signature(&self) -> Signature {
        Signature::build("mkdir").rest(SyntaxShape::Path, "the name(s) of the path(s) to create")
    }

    fn usage(&self) -> &str {
        "Make directories, creates intermediary directories as required."
    }

    fn run(
        &self,
        call_info: &CallInfo,
        _registry: &CommandRegistry,
        raw_args: &RawCommandArgs,
        _input: Value,
    ) -> Result<OutputStream, ShellError> {
        call_info.process(&raw_args.shell_manager, mkdir)?.run()
    }
}

fn mkdir(args: MkdirArgs, context: &RunnablePerItemContext) -> Result<OutputStream, ShellError> {
    let shell_manager = context.shell_manager.clone();
    shell_manager.mkdir(args, context)
}
