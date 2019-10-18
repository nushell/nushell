use crate::commands::command::RunnablePerItemContext;
use crate::errors::ShellError;
use crate::parser::hir::SyntaxShape;
use crate::parser::registry::{CommandRegistry, Signature};
use crate::prelude::*;
use std::path::PathBuf;

pub struct Remove;

#[derive(Deserialize)]
pub struct RemoveArgs {
    pub target: Tagged<PathBuf>,
    pub recursive: Tagged<bool>,
    pub trash: Tagged<bool>,
}

impl PerItemCommand for Remove {
    fn name(&self) -> &str {
        "rm"
    }

    fn signature(&self) -> Signature {
        Signature::build("rm")
            .required("path", SyntaxShape::Pattern)
            .switch("trash")
            .switch("recursive")
    }

    fn usage(&self) -> &str {
        "Remove a file. Append '--recursive' to remove directories and '--trash' for seding it to system recycle bin"
    }

    fn run(
        &self,
        call_info: &CallInfo,
        _registry: &CommandRegistry,
        raw_args: &RawCommandArgs,
        _input: Tagged<Value>,
    ) -> Result<OutputStream, ShellError> {
        call_info.process(&raw_args.shell_manager, rm)?.run()
    }
}

fn rm(args: RemoveArgs, context: &RunnablePerItemContext) -> Result<OutputStream, ShellError> {
    let shell_manager = context.shell_manager.clone();
    shell_manager.rm(args, context)
}
