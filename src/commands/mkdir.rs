use crate::errors::ShellError;
use crate::parser::hir::SyntaxType;
use crate::parser::registry::{CommandRegistry, Signature};
use crate::prelude::*;
use std::path::{Path, PathBuf};

pub struct Mkdir;

impl PerItemCommand for Mkdir {
    fn run(
        &self,
        call_info: &CallInfo,
        registry: &CommandRegistry,
        shell_manager: &ShellManager,
        input: Tagged<Value>,
    ) -> Result<VecDeque<ReturnValue>, ShellError> {
        mkdir(call_info, registry, shell_manager, input)
    }

    fn name(&self) -> &str {
        "mkdir"
    }

    fn signature(&self) -> Signature {
        Signature::build("mkdir").named("file", SyntaxType::Any)
    }
}

pub fn mkdir(
    call_info: &CallInfo,
    _registry: &CommandRegistry,
    shell_manager: &ShellManager,
    _input: Tagged<Value>,
) -> Result<VecDeque<ReturnValue>, ShellError> {
    let mut full_path = PathBuf::from(shell_manager.path());

    match &call_info.args.nth(0) {
        Some(Tagged { item: value, .. }) => full_path.push(Path::new(&value.as_string()?)),
        _ => {}
    }

    match std::fs::create_dir_all(full_path) {
        Err(reason) => Err(ShellError::labeled_error(
            reason.to_string(),
            reason.to_string(),
            call_info.args.nth(0).unwrap().span(),
        )),
        Ok(_) => Ok(VecDeque::new()),
    }
}
