use crate::errors::ShellError;
use crate::parser::hir::SyntaxType;
use crate::parser::registry::{CommandRegistry, Signature};
use crate::prelude::*;
use std::path::{Path, PathBuf};

pub struct Mkdir;

impl StaticCommand for Mkdir {
    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        mkdir(args, registry)
    }

    fn name(&self) -> &str {
        "mkdir"
    }

    fn signature(&self) -> Signature {
        Signature::build("mkdir").named("file", SyntaxType::Any)
    }
}

pub fn mkdir(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;

    let mut full_path = PathBuf::from(args.shell_manager.path());

    match &args.nth(0) {
        Some(Tagged { item: value, .. }) => full_path.push(Path::new(&value.as_string()?)),
        _ => {}
    }

    match std::fs::create_dir_all(full_path) {
        Err(reason) => Err(ShellError::labeled_error(
            reason.to_string(),
            reason.to_string(),
            args.nth(0).unwrap().span(),
        )),
        Ok(_) => Ok(OutputStream::empty()),
    }
}
