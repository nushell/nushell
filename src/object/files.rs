use crate::errors::ShellError;
use crate::object::{Dictionary, Value};

#[derive(Debug)]
pub enum FileType {
    Directory,
    File,
    Symlink,
}

crate fn dir_entry_dict(entry: &std::fs::DirEntry) -> Result<Dictionary, ShellError> {
    let mut dict = Dictionary::default();
    let filename = entry.file_name();
    dict.add("file name", Value::string(filename.to_string_lossy()));

    let metadata = entry.metadata()?;

    let kind = if metadata.is_dir() {
        FileType::Directory
    } else if metadata.is_file() {
        FileType::File
    } else {
        FileType::Symlink
    };

    dict.add("file type", Value::string(format!("{:?}", kind)));
    dict.add(
        "readonly",
        Value::boolean(metadata.permissions().readonly()),
    );

    dict.add("size", Value::bytes(metadata.len() as u64));

    match metadata.created() {
        Ok(c) => dict.add("created", Value::system_date(c)),
        Err(_) => {}
    }

    match metadata.accessed() {
        Ok(a) => dict.add("accessed", Value::system_date(a)),
        Err(_) => {}
    }

    match metadata.modified() {
        Ok(m) => dict.add("modified", Value::system_date(m)),
        Err(_) => {}
    }

    Ok(dict)
}
