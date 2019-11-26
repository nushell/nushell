use crate::data::TaggedDictBuilder;
use crate::prelude::*;
use nu_protocol::{Value};
use nu_errors::ShellError;

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
) -> Result<Value, ShellError> {
    let mut dict = TaggedDictBuilder::new(tag);
    dict.insert_untagged("name", value::string(filename.to_string_lossy()));

    let kind = if metadata.is_dir() {
        FileType::Directory
    } else if metadata.is_file() {
        FileType::File
    } else {
        FileType::Symlink
    };

    dict.insert_untagged("type", value::string(format!("{:?}", kind)));

    if full {
        dict.insert_untagged(
            "readonly",
            value::boolean(metadata.permissions().readonly()),
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = metadata.permissions().mode();
            dict.insert_untagged("mode", value::string(umask::Mode::from(mode).to_string()));
        }
    }

    dict.insert_untagged("size", value::bytes(metadata.len() as u64));

    match metadata.created() {
        Ok(c) => dict.insert_untagged("created", value::system_date(c)),
        Err(_) => {}
    }

    match metadata.accessed() {
        Ok(a) => dict.insert_untagged("accessed", value::system_date(a)),
        Err(_) => {}
    }

    match metadata.modified() {
        Ok(m) => dict.insert_untagged("modified", value::system_date(m)),
        Err(_) => {}
    }

    Ok(dict.into_value())
}
