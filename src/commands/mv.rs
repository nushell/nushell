use crate::commands::command::RunnablePerItemContext;
use crate::errors::ShellError;
use crate::parser::hir::SyntaxType;
use crate::parser::registry::{CommandRegistry, Signature};
use crate::prelude::*;
use std::path::PathBuf;

pub struct Move;

#[derive(Deserialize)]
pub struct MoveArgs {
    pub src: Tagged<PathBuf>,
    pub dst: Tagged<PathBuf>,
}

impl PerItemCommand for Move {
    fn name(&self) -> &str {
        "mv"
    }

    fn signature(&self) -> Signature {
        Signature::build("mv")
            .required("source", SyntaxType::Path)
            .required("destination", SyntaxType::Path)
            .named("file", SyntaxType::Any)
    }

    fn run(
        &self,
        call_info: &CallInfo,
        _registry: &CommandRegistry,
        shell_manager: &ShellManager,
        _input: Tagged<Value>,
    ) -> Result<VecDeque<ReturnValue>, ShellError> {
        call_info.process(shell_manager, mv)?.run()
    }
}

fn mv(
    args: MoveArgs,
    context: &RunnablePerItemContext,
) -> Result<VecDeque<ReturnValue>, ShellError> {
    let shell_manager = context.shell_manager.clone();
    shell_manager.mv(args, context)
}
