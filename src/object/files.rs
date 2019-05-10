use crate::errors::ShellError;
use crate::object::{DataDescriptor, Dictionary, ShellObject, Value};
use crate::MaybeOwned;

#[derive(Debug)]
pub struct DirEntry {
    inner: std::fs::DirEntry,
    dict: Dictionary,
}

#[derive(Debug)]
pub enum FileType {
    Dir,
    File,
    Symlink,
}

impl DirEntry {
    crate fn new(inner: std::fs::DirEntry) -> Result<DirEntry, ShellError> {
        let mut dict = Dictionary::default();
        let filename = inner.file_name();
        dict.add("file_name", Value::string(filename.to_string_lossy()));

        let metadata = inner.metadata()?;
        // let file_type = inner.file_type()?;

        let kind = if metadata.is_dir() {
            FileType::Dir
        } else if metadata.is_file() {
            FileType::File
        } else {
            FileType::Symlink
        };

        dict.add("file_type", Value::string(format!("{:?}", kind)));
        dict.add(
            "readonly",
            Value::boolean(metadata.permissions().readonly()),
        );

        Ok(DirEntry { inner, dict })
    }
}

impl ShellObject for DirEntry {
    fn to_shell_string(&self) -> String {
        format!("[object DirEntry]")
    }

    fn data_descriptors(&self) -> Vec<DataDescriptor> {
        self.dict.data_descriptors()
    }

    fn get_data(&'a self, desc: &DataDescriptor) -> MaybeOwned<'a, Value> {
        self.dict.get_data(desc)
    }
}
