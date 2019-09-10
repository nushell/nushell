use crate::commands::command::RunnablePerItemContext;
use crate::errors::ShellError;
use crate::parser::hir::SyntaxType;
use crate::parser::registry::{CommandRegistry, Signature};
use crate::prelude::*;
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
            .required("src", SyntaxType::Pattern)
            .required("dst", SyntaxType::Path)
            .named("file", SyntaxType::Any)
            .switch("recursive")
    }

    fn usage(&self) -> &str {
        "Copy files."
    }

    fn run(
        &self,
        call_info: &CallInfo,
        _registry: &CommandRegistry,
        raw_args: &RawCommandArgs,
        _input: Tagged<Value>,
    ) -> Result<OutputStream, ShellError> {
        call_info.process(&raw_args.shell_manager, cp)?.run()
    }
}

fn cp(args: CopyArgs, context: &RunnablePerItemContext) -> Result<OutputStream, ShellError> {
    let shell_manager = context.shell_manager.clone();
    shell_manager.cp(args, context)
}
