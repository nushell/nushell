use crate::errors::ShellError;
use crate::object::{dir_entry_dict, Primitive, Value};
use crate::parser::lexer::Spanned;
use crate::prelude::*;
use std::path::{Path, PathBuf};

pub fn ls(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let cwd = args.env.lock().unwrap().cwd().to_path_buf();
    let mut full_path = PathBuf::from(cwd);
    match &args.positional.get(0) {
        Some(Spanned {
            item: Value::Primitive(Primitive::String(s)),
            ..
        }) => full_path.push(Path::new(s)),
        _ => {}
    }

    let entries = std::fs::read_dir(&full_path);

    let entries = match entries {
        Err(e) => {
            if let Some(s) = args.positional.get(0) {
                return Err(ShellError::labeled_error(
                    e.to_string(),
                    e.to_string(),
                    s.span,
                ));
            } else {
                return Err(ShellError::string(e.to_string()));
            }
        }
        Ok(o) => o,
    };

    let mut shell_entries = VecDeque::new();

    for entry in entries {
        let value = Value::Object(dir_entry_dict(&entry?)?);
        shell_entries.push_back(ReturnValue::Value(value))
    }

    Ok(shell_entries.boxed())
}
