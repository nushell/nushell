use crate::commands::command::RunnablePerItemContext;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{CallInfo, Signature, SyntaxShape, Value};
use nu_source::Tagged;
use std::path::PathBuf;

pub struct Cpy;

#[derive(Deserialize)]
pub struct CopyArgs {
    pub src: Tagged<PathBuf>,
    pub dst: Tagged<PathBuf>,
    pub recursive: Tagged<bool>,
}

impl PerItemCommand for Cpy {
    fn name(&self) -> &str {
        "cp"
    }

    fn signature(&self) -> Signature {
        Signature::build("cp")
            .required("src", SyntaxShape::Pattern, "the place to copy from")
            .required("dst", SyntaxShape::Path, "the place to copy to")
            .switch("recursive", "copy recursively through subdirectories")
    }

    fn usage(&self) -> &str {
        "Copy files."
    }

    fn run(
        &self,
        call_info: &CallInfo,
        _registry: &CommandRegistry,
        raw_args: &RawCommandArgs,
        _input: Value,
    ) -> Result<OutputStream, ShellError> {
        call_info.process(&raw_args.shell_manager, cp)?.run()
    }
}

fn cp(args: CopyArgs, context: &RunnablePerItemContext) -> Result<OutputStream, ShellError> {
    let shell_manager = context.shell_manager.clone();
    shell_manager.cp(args, context)
}
