use crate::errors::ShellError;
use crate::object::dir_entry_dict;
use crate::prelude::*;
use std::path::{Path, PathBuf};

pub fn ls(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let env = args.env.lock().unwrap();
    let path = env.path.to_path_buf();
    let cwd = path.clone();
    let mut full_path = PathBuf::from(path);
    match &args.nth(0) {
        Some(Tagged { item: value, .. }) => full_path.push(Path::new(&value.as_string()?)),
        _ => {}
    }

    let entries = glob::glob(&full_path.to_string_lossy());

    if entries.is_err() {
        return Err(ShellError::string("Invalid pattern."));
    }

    let mut shell_entries = VecDeque::new();
    let entries: Vec<_> = entries.unwrap().collect();

    // If this is a single entry, try to display the contents of the entry if it's a directory
    if entries.len() == 1 {
        if let Ok(entry) = &entries[0] {
            if entry.is_dir() {
                let entries = std::fs::read_dir(&full_path);

                let entries = match entries {
                    Err(e) => {
                        if let Some(s) = args.nth(0) {
                            return Err(ShellError::labeled_error(
                                e.to_string(),
                                e.to_string(),
                                s.span(),
                            ));
                        } else {
                            return Err(ShellError::maybe_labeled_error(
                                e.to_string(),
                                e.to_string(),
                                args.call_info.name_span,
                            ));
                        }
                    }
                    Ok(o) => o,
                };
                for entry in entries {
                    let entry = entry?;
                    let filepath = entry.path();
                    let filename = filepath.strip_prefix(&cwd).unwrap();
                    let value =
                        dir_entry_dict(filename, &entry.metadata()?, args.call_info.name_span)?;
                    shell_entries.push_back(ReturnSuccess::value(value))
                }
                return Ok(shell_entries.to_output_stream());
            }
        }
    }

    // Enumerate the entries from the glob and add each
    for entry in entries {
        if let Ok(entry) = entry {
            let filename = entry.strip_prefix(&cwd).unwrap();
            let metadata = std::fs::metadata(&entry)?;
            let value = dir_entry_dict(filename, &metadata, args.call_info.name_span)?;
            shell_entries.push_back(ReturnSuccess::value(value))
        }
    }

    Ok(shell_entries.to_output_stream())
}
