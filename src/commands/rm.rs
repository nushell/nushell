use crate::commands::StaticCommand;
use crate::errors::ShellError;
use crate::parser::hir::SyntaxType;
use crate::prelude::*;
use std::path::PathBuf;

pub struct Remove;

#[derive(Deserialize)]
pub struct RemoveArgs {
    path: Spanned<PathBuf>,
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

    if full_path.is_dir() {
        if !recursive {
            return Err(ShellError::maybe_labeled_error(
                "is a directory",
                "",
                context.name,
            ));
        }
        std::fs::remove_dir_all(&full_path).expect("can not remove directory");
    } else if full_path.is_file() {
        std::fs::remove_file(&full_path).expect("can not remove file");
    }

    Ok(OutputStream::empty())
}
