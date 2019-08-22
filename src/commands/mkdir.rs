use crate::commands::command::RunnablePerItemContext;
use crate::errors::ShellError;
use crate::parser::registry::{CommandRegistry, Signature};
use crate::prelude::*;
use std::path::PathBuf;

pub struct Mkdir;

#[derive(Deserialize)]
pub struct MkdirArgs {
    pub rest: Vec<Tagged<PathBuf>>,
}

impl PerItemCommand for Mkdir {
    fn run(
        &self,
        call_info: &CallInfo,
        _registry: &CommandRegistry,
        shell_manager: &ShellManager,
        _input: Tagged<Value>,
    ) -> Result<VecDeque<ReturnValue>, ShellError> {
        call_info.process(shell_manager, mkdir)?.run()
    }

    fn name(&self) -> &str {
        "mkdir"
    }

    fn signature(&self) -> Signature {
        Signature::build("mkdir").rest()
    }
}

fn mkdir(
    args: MkdirArgs,
    context: &RunnablePerItemContext,
) -> Result<VecDeque<ReturnValue>, ShellError> {
    let shell_manager = context.shell_manager.clone();
    shell_manager.mkdir(args, context)
}
