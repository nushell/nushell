use crate::commands::StaticCommand;
use crate::errors::ShellError;
use crate::parser::hir::SyntaxType;
use crate::prelude::*;

use glob::glob;
use std::path::PathBuf;

pub struct Remove;

#[derive(Deserialize)]
pub struct RemoveArgs {
    path: Tagged<PathBuf>,
    recursive: bool,
}

impl StaticCommand for Remove {
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
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, rm)?.run()
    }
}

pub fn rm(
    RemoveArgs { path, recursive }: RemoveArgs,
    context: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let mut full_path = context.cwd();

    match path.item.to_str().unwrap() {
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
                    if !recursive {
                        return Err(ShellError::string(
                            "is a directory",
                            // "is a directory",
                            // args.call_info.name_span,
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

    Ok(OutputStream::empty())
}
