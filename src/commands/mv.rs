use crate::commands::command::RunnablePerItemContext;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{CallInfo, Signature, SyntaxShape, Value};
use nu_source::Tagged;
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
            .required(
                "source",
                SyntaxShape::Pattern,
                "the location to move files/directories from",
            )
            .required(
                "destination",
                SyntaxShape::Path,
                "the location to move files/directories to",
            )
    }

    fn usage(&self) -> &str {
        "Move files or directories."
    }

    fn run(
        &self,
        call_info: &CallInfo,
        _registry: &CommandRegistry,
        raw_args: &RawCommandArgs,
        _input: Value,
    ) -> Result<OutputStream, ShellError> {
        call_info.process(&raw_args.shell_manager, mv)?.run()
    }
}

fn mv(args: MoveArgs, context: &RunnablePerItemContext) -> Result<OutputStream, ShellError> {
    let shell_manager = context.shell_manager.clone();
    shell_manager.mv(args, context)
}
