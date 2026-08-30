use nu_path::{AbsolutePath, Path};

pub enum Stub<'a> {
    FileWithContent(&'a str, &'a str),
    FileWithContentToBeTrimmed(&'a str, &'a str),
    EmptyFile(&'a str),
    FileWithPermission(&'a str, bool),
}

pub fn files_exist_at(files: &[impl AsRef<Path>], path: impl AsRef<AbsolutePath>) -> bool {
    let path = path.as_ref();
    files.iter().all(|f| path.join(f.as_ref()).exists())
}
