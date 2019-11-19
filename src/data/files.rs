use crate::data::{TaggedDictBuilder, Value};
use crate::errors::ShellError;
use crate::prelude::*;

#[derive(Debug)]
pub enum FileType {
    Directory,
    File,
    Symlink,
}

pub(crate) fn dir_entry_dict(
    filename: &std::path::Path,
    metadata: &std::fs::Metadata,
    tag: impl Into<Tag>,
    full: bool,
) -> Result<Tagged<Value>, ShellError> {
    let mut dict = TaggedDictBuilder::new(tag);
    dict.insert("name", Value::string(filename.to_string_lossy()));

    let kind = if metadata.is_dir() {
        FileType::Directory
    } else if metadata.is_file() {
        FileType::File
    } else {
        FileType::Symlink
    };

    dict.insert("type", Value::string(format!("{:?}", kind)));

    if full {
        dict.insert(
            "readonly",
            Value::boolean(metadata.permissions().readonly()),
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = metadata.permissions().mode();
            dict.insert("mode", Value::string(umask::Mode::from(mode).to_string()));
        }
    }

    dict.insert("size", Value::bytes(metadata.len() as u64));

    match metadata.created() {
        Ok(c) => dict.insert("created", Value::system_date(c)),
        Err(_) => {}
    }

    match metadata.accessed() {
        Ok(a) => dict.insert("accessed", Value::system_date(a)),
        Err(_) => {}
    }

    match metadata.modified() {
        Ok(m) => dict.insert("modified", Value::system_date(m)),
        Err(_) => {}
    }

    Ok(dict.into_tagged_value())
}
