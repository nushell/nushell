use crate::errors::ShellError;
use crate::object::{dir_entry_dict, Primitive, Value};
use crate::parser::Spanned;
use crate::prelude::*;
use std::path::{Path, PathBuf};

pub fn ls(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let env = args.env.lock().unwrap();
    let path = env.path.to_path_buf();
    let mut full_path = PathBuf::from(path);
    match &args.nth(0) {
        Some(Spanned {
            item: Value::Primitive(Primitive::String(s)),
            ..
        }) => full_path.push(Path::new(&s)),
        _ => {}
    }

    let entries = std::fs::read_dir(&full_path);

    let entries = match entries {
        Err(e) => {
            if let Some(s) = args.nth(0) {
                return Err(ShellError::labeled_error(
                    e.to_string(),
                    e.to_string(),
                    s.span,
                ));
            } else {
                return Err(ShellError::maybe_labeled_error(
                    e.to_string(),
                    e.to_string(),
                    args.name_span,
                ));
            }
        }
        Ok(o) => o,
    };

    let mut shell_entries = VecDeque::new();

    for entry in entries {
        let value = dir_entry_dict(&entry?, args.name_span)?;
        shell_entries.push_back(ReturnSuccess::value(value))
    }
    Ok(shell_entries.to_output_stream())
}
