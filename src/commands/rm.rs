use crate::errors::ShellError;
use crate::parser::hir::SyntaxType;
use crate::parser::registry::{CommandConfig, NamedType, PositionalType};
use crate::prelude::*;
use indexmap::IndexMap;

pub struct Remove;

impl Command for Remove {
    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        rm(args)
    }

    fn name(&self) -> &str {
        "rm"
    }

    fn config(&self) -> CommandConfig {
        let mut named: IndexMap<String, NamedType> = IndexMap::new();
        named.insert("recursive".to_string(), NamedType::Switch);

        CommandConfig {
            name: self.name().to_string(),
            positional: vec![PositionalType::mandatory("file", SyntaxType::Path)],
            rest_positional: false,
            named,
            is_sink: true,
            is_filter: false,
        }
    }
}

pub fn rm(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let mut full_path = args.env.lock().unwrap().path().to_path_buf();

    match args
        .nth(0)
        .ok_or_else(|| ShellError::string(&format!("No file or directory specified")))?
        .as_string()?
        .as_str()
    {
        "." | ".." => return Err(ShellError::string("\".\" and \"..\" may not be removed")),
        file => full_path.push(file),
    }

    if full_path.is_dir() {
        if !args.has("recursive") {
            return Err(ShellError::labeled_error(
                "is a directory",
                "",
                args.call_info.name_span.unwrap(),
            ));
        }
        std::fs::remove_dir_all(&full_path).expect("can not remove directory");
    } else if full_path.is_file() {
        std::fs::remove_file(&full_path).expect("can not remove file");
    }

    Ok(OutputStream::empty())
}
