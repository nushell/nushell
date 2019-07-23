use crate::errors::ShellError;
use crate::parser::hir::SyntaxType;
use crate::parser::registry::{CommandConfig, NamedType, PositionalType};
use crate::prelude::*;
use indexmap::IndexMap;
use std::path::Path;

pub struct Copycp;

impl Command for Copycp {
    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        cp(args)
    }

    fn name(&self) -> &str {
        "cp"
    }

    fn config(&self) -> CommandConfig {
        let mut named: IndexMap<String, NamedType> = IndexMap::new();
        named.insert("recursive".to_string(), NamedType::Switch);

        CommandConfig {
            name: self.name().to_string(),
            positional: vec![PositionalType::mandatory("file", SyntaxType::Path)],
            rest_positional: false,
            named,
            is_sink: false,
            is_filter: false,
        }
    }
}

pub fn cp(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let mut source = args.env.lock().unwrap().path().to_path_buf();
    let mut destination = args.env.lock().unwrap().path().to_path_buf();

    let mut dst = String::new();

    match args
        .nth(0)
        .ok_or_else(|| ShellError::string(&format!("No file or directory specified")))?
        .as_string()?
        .as_str()
    {
        file => {
            source.push(file);
        }
    }

    match args
        .nth(1)
        .ok_or_else(|| ShellError::string(&format!("No file or directory specified")))?
        .as_string()?
        .as_str()
    {
        file => {
            dst.push_str(file);
            destination.push(file);
        }
    }

    if destination.is_dir() {
        if source.is_file() {
            let file_name = source.file_name().expect("");
            let file_name = file_name.to_str().expect("");
            destination.push(Path::new(file_name));
        } else if source.is_dir() {
            return Err(ShellError::string(&format!(
                "{:?} is a directory (not copied)",
                source.to_string_lossy()
            )));
        }
    }

    match std::fs::copy(source, destination) {
        Err(_error) => Err(ShellError::string("can not copy file")),
        Ok(_) => Ok(OutputStream::empty()),
    }
}
