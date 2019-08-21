use crate::commands::command::RunnablePerItemContext;
use crate::errors::ShellError;
use crate::parser::hir::SyntaxType;
use crate::parser::registry::{CommandRegistry, Signature};
use crate::prelude::*;
use crate::utils::FileStructure;
use std::path::PathBuf;

pub struct Remove;

#[derive(Deserialize)]
pub struct RemoveArgs {
    target: Tagged<PathBuf>,
    recursive: Tagged<bool>,
}

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
        call_info.process(shell_manager, rm)?.run()
    }
}

fn rm(
    RemoveArgs { target, recursive }: RemoveArgs,
    RunnablePerItemContext { name, .. }: &RunnablePerItemContext,
) -> Result<VecDeque<ReturnValue>, ShellError> {
    let path = target.item.clone();
    let name_span = name;

    let file = path.to_string_lossy();

    if file == "." || file == ".." {
        return Err(ShellError::labeled_error(
            "Remove aborted. \".\" or \"..\" may not be removed.",
            "Remove aborted. \".\" or \"..\" may not be removed.",
            target.span(),
        ));
    }

    let entries: Vec<_> = match glob::glob(&path.to_string_lossy()) {
        Ok(files) => files.collect(),
        Err(_) => {
            return Err(ShellError::labeled_error(
                "Invalid pattern.",
                "Invalid pattern.",
                target.tag,
            ))
        }
    };

    if entries.len() == 1 {
        if let Ok(entry) = &entries[0] {
            if entry.is_dir() {
                let mut source_dir: FileStructure = FileStructure::new();

                source_dir.walk_decorate(&entry)?;

                if source_dir.contains_files() && !recursive.item {
                    return Err(ShellError::labeled_error(
                        format!("{:?} is a directory. Try using \"--recursive\".", file),
                        format!("{:?} is a directory. Try using \"--recursive\".", file),
                        target.span(),
                    ));
                }
            }
        }
    }

    for entry in entries {
        match entry {
            Ok(path) => {
                let path_file_name = {
                    match path.file_name() {
                        Some(name) => PathBuf::from(name),
                        None => {
                            return Err(ShellError::labeled_error(
                                "Remove aborted. Not a valid path",
                                "Remove aborted. Not a valid path",
                                name_span,
                            ))
                        }
                    }
                };

                let mut source_dir: FileStructure = FileStructure::new();

                source_dir.walk_decorate(&path)?;

                if source_dir.contains_more_than_one_file() && !recursive.item {
                    return Err(ShellError::labeled_error(
                        format!(
                            "Directory {:?} found somewhere inside. Try using \"--recursive\".",
                            path_file_name
                        ),
                        format!(
                            "Directory {:?} found somewhere inside. Try using \"--recursive\".",
                            path_file_name
                        ),
                        target.span(),
                    ));
                }

                if path.is_dir() {
                    std::fs::remove_dir_all(&path)?;
                } else if path.is_file() {
                    std::fs::remove_file(&path)?;
                }
            }
            Err(e) => {
                return Err(ShellError::labeled_error(
                    format!("Remove aborted. {:}", e.to_string()),
                    format!("Remove aborted. {:}", e.to_string()),
                    name_span,
                ))
            }
        }
    }

    Ok(VecDeque::new())
}
