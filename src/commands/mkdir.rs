use crate::errors::ShellError;
use crate::parser::hir::SyntaxType;
use crate::parser::registry::{CommandConfig, NamedType, PositionalType};
use crate::prelude::*;
use indexmap::IndexMap;
use std::path::{Path, PathBuf};

pub struct Mkdir;

impl Command for Mkdir {
    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        mkdir(args)
    }

    fn name(&self) -> &str {
        "mkdir"
    }

    fn config(&self) -> CommandConfig {
        let mut named: IndexMap<String, NamedType> = IndexMap::new();
        named.insert("p".to_string(), NamedType::Switch);

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

pub fn mkdir(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let env = args.env.lock().unwrap();
    let path = env.path.to_path_buf();
    let cwd = path.clone();
    let mut full_path = PathBuf::from(path);

    match &args.nth(0) {
        Some(Tagged { item: value, .. }) => full_path.push(Path::new(&value.as_string()?)),
        _ => {}
    }

    if !args.has("p") {
        match std::fs::create_dir(full_path) {
            Err(_) => Err(ShellError::labeled_error(
                "No such file or directory",
                "No such file or directory",
                args.nth(0).unwrap().span(),
            )),
            Ok(_) => Ok(OutputStream::empty()),
        }
    } else {
        match std::fs::create_dir_all(full_path) {
            Err(reason) => Err(ShellError::labeled_error(
                reason.to_string(),
                reason.to_string(),
                args.nth(0).unwrap().span(),
            )),
            Ok(_) => Ok(OutputStream::empty()),
        }
    }
}
