use crate::errors::ShellError;
use crate::parser::hir::SyntaxType;
use crate::prelude::*;

use glob::glob;
use std::path::PathBuf;

pub struct Remove;

impl PerItemCommand for Remove {
    fn name(&self) -> &str {
        "rm"
    }

    fn signature(&self) -> Signature {
        Signature::build("rm")
            .required("path", SyntaxType::Path)
            .switch("recursive")
    }

    fn run(
        &self,
        call_info: &CallInfo,
        _registry: &CommandRegistry,
        shell_manager: &ShellManager,
        _input: Tagged<Value>,
    ) -> Result<VecDeque<ReturnValue>, ShellError> {
        rm(call_info, shell_manager)
    }
}

pub fn rm(
    call_info: &CallInfo,
    shell_manager: &ShellManager,
) -> Result<VecDeque<ReturnValue>, ShellError> {
    let mut full_path = PathBuf::from(shell_manager.path());

    match call_info
        .args
        .nth(0)
        .ok_or_else(|| ShellError::string(&format!("No file or directory specified")))?
        .as_string()?
        .as_str()
    {
        "." | ".." => return Err(ShellError::string("\".\" and \"..\" may not be removed")),
        file => full_path.push(file),
    }

    let entries = glob(&full_path.to_string_lossy());

    if entries.is_err() {
        return Err(ShellError::string("Invalid pattern."));
    }

    let entries = entries.unwrap();

    for entry in entries {
        match entry {
            Ok(path) => {
                if path.is_dir() {
                    if !call_info.args.has("recursive") {
                        return Err(ShellError::labeled_error(
                            "is a directory",
                            "is a directory",
                            call_info.name_span,
                        ));
                    }
                    std::fs::remove_dir_all(&path).expect("can not remove directory");
                } else if path.is_file() {
                    std::fs::remove_file(&path).expect("can not remove file");
                }
            }
            Err(e) => return Err(ShellError::string(&format!("{:?}", e))),
        }
    }

    Ok(VecDeque::new())
}
