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

/// Every `.nu` file under `dir`, in a stable order: each directory's entries sorted, depth first.
///
/// Panics if a directory can't be read.
pub fn nu_files(dir: impl AsRef<std::path::Path>) -> Vec<std::path::PathBuf> {
    fn collect(dir: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .unwrap_or_else(|err| panic!("cannot read {}: {err}", dir.display()))
            .map(|entry| entry.expect("directory entry is readable").path())
            .collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                collect(&path, files);
            } else if path.extension().is_some_and(|ext| ext == "nu") {
                files.push(path);
            }
        }
    }
    let mut files = vec![];
    collect(dir.as_ref(), &mut files);
    files
}
