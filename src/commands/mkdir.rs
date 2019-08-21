use crate::commands::command::RunnablePerItemContext;
use crate::errors::ShellError;
use crate::parser::registry::{CommandRegistry, Signature};
use crate::prelude::*;
use std::path::PathBuf;

pub struct Mkdir;

#[derive(Deserialize)]
struct MkdirArgs {
    rest: Vec<Tagged<PathBuf>>,
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
    MkdirArgs { rest: directories }: MkdirArgs,
    RunnablePerItemContext {
        name,
        shell_manager,
        ..
    }: &RunnablePerItemContext,
) -> Result<VecDeque<ReturnValue>, ShellError> {
    let full_path = PathBuf::from(shell_manager.path());

    if directories.len() == 0 {
        return Err(ShellError::labeled_error(
            "mkdir requires directory paths",
            "needs parameter",
            name,
        ));
    }

    for dir in directories.iter() {
        let create_at = {
            let mut loc = full_path.clone();
            loc.push(&dir.item);
            loc
        };

        match std::fs::create_dir_all(create_at) {
            Err(reason) => {
                return Err(ShellError::labeled_error(
                    reason.to_string(),
                    reason.to_string(),
                    dir.span(),
                ))
            }
            Ok(_) => {}
        }
    }

    Ok(VecDeque::new())
}
