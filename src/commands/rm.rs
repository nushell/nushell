use crate::commands::command::RunnablePerItemContext;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{CallInfo, Signature, SyntaxShape, Value};
use nu_source::Tagged;
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
            .required("path", SyntaxShape::Pattern, "the file path to remove")
            .switch(
                "trash",
                "use the platform's recycle bin instead of permanently deleting",
            )
            .switch("recursive", "delete subdirectories recursively")
    }

    fn usage(&self) -> &str {
        "Remove a file"
    }

    fn run(
        &self,
        call_info: &CallInfo,
        _registry: &CommandRegistry,
        raw_args: &RawCommandArgs,
        _input: Value,
    ) -> Result<OutputStream, ShellError> {
        call_info.process(&raw_args.shell_manager, rm)?.run()
    }
}

fn rm(args: RemoveArgs, context: &RunnablePerItemContext) -> Result<OutputStream, ShellError> {
    let shell_manager = context.shell_manager.clone();
    shell_manager.rm(args, context)
}
