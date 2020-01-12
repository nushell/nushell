use std::io::Read;
use std::ops::Div;
use std::path::{Path, PathBuf};

pub struct AbsoluteFile {
    inner: PathBuf,
}

impl AbsoluteFile {
    pub fn new(path: impl AsRef<Path>) -> AbsoluteFile {
        let path = path.as_ref();

        if !path.is_absolute() {
            panic!(
                "AbsoluteFile::new must take an absolute path :: {}",
                path.display()
            )
        } else if path.is_dir() {
            // At the moment, this is not an invariant, but rather a way to catch bugs
            // in tests.
            panic!(
                "AbsoluteFile::new must not take a directory :: {}",
                path.display()
            )
        } else {
            AbsoluteFile {
                inner: path.to_path_buf(),
            }
        }
    }

    pub fn dir(&self) -> AbsolutePath {
        AbsolutePath::new(if let Some(parent) = self.inner.parent() {
            parent
        } else {
            unreachable!("Internal error: could not get parent in dir")
        })
    }
}

impl From<AbsoluteFile> for PathBuf {
    fn from(file: AbsoluteFile) -> Self {
        file.inner
    }
}

pub struct AbsolutePath {
    inner: PathBuf,
}

impl AbsolutePath {
    pub fn new(path: impl AsRef<Path>) -> AbsolutePath {
        let path = path.as_ref();

        if path.is_absolute() {
            AbsolutePath {
                inner: path.to_path_buf(),
            }
        } else {
            panic!("AbsolutePath::new must take an absolute path")
        }
    }
}

impl Div<&str> for &AbsolutePath {
    type Output = AbsolutePath;

    fn div(self, rhs: &str) -> Self::Output {
        let parts = rhs.split('/');
        let mut result = self.inner.clone();

        for part in parts {
            result = result.join(part);
        }

        AbsolutePath::new(result)
    }
}

impl AsRef<Path> for AbsolutePath {
    fn as_ref(&self) -> &Path {
        self.inner.as_path()
    }
}

pub struct RelativePath {
    inner: PathBuf,
}

impl RelativePath {
    pub fn new(path: impl Into<PathBuf>) -> RelativePath {
        let path = path.into();

        if path.is_relative() {
            RelativePath { inner: path }
        } else {
            panic!("RelativePath::new must take a relative path")
        }
    }
}

impl<T: AsRef<str>> Div<T> for &RelativePath {
    type Output = RelativePath;

    fn div(self, rhs: T) -> Self::Output {
        let parts = rhs.as_ref().split('/');
        let mut result = self.inner.clone();

        for part in parts {
            result = result.join(part);
        }

        RelativePath::new(result)
    }
}
pub trait DisplayPath {
    fn display_path(&self) -> String;
}

impl DisplayPath for AbsolutePath {
    fn display_path(&self) -> String {
        self.inner.display().to_string()
    }
}

impl DisplayPath for PathBuf {
    fn display_path(&self) -> String {
        self.display().to_string()
    }
}

impl DisplayPath for str {
    fn display_path(&self) -> String {
        self.to_string()
    }
}

impl DisplayPath for &str {
    fn display_path(&self) -> String {
        (*self).to_string()
    }
}

impl DisplayPath for String {
    fn display_path(&self) -> String {
        self.clone()
    }
}

impl DisplayPath for &String {
    fn display_path(&self) -> String {
        (*self).to_string()
    }
}
pub enum Stub<'a> {
    FileWithContent(&'a str, &'a str),
    FileWithContentToBeTrimmed(&'a str, &'a str),
    EmptyFile(&'a str),
}

pub fn file_contents(full_path: impl AsRef<Path>) -> String {
    let mut file = std::fs::File::open(full_path.as_ref()).expect("can not open file");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("can not read file");
    contents
}

pub fn file_contents_binary(full_path: impl AsRef<Path>) -> Vec<u8> {
    let mut file = std::fs::File::open(full_path.as_ref()).expect("can not open file");
    let mut contents = Vec::new();
    file.read_to_end(&mut contents).expect("can not read file");
    contents
}

pub fn line_ending() -> String {
    #[cfg(windows)]
    {
        String::from("\r\n")
    }

    #[cfg(not(windows))]
    {
        String::from("\n")
    }
}

pub fn delete_file_at(full_path: impl AsRef<Path>) {
    let full_path = full_path.as_ref();

    if full_path.exists() {
        std::fs::remove_file(full_path).expect("can not delete file");
    }
}

pub fn create_file_at(full_path: impl AsRef<Path>) -> Result<(), std::io::Error> {
    let full_path = full_path.as_ref();

    if let Some(parent) = full_path.parent() {
        panic!(format!("{:?} exists", parent.display()));
    }

    std::fs::write(full_path, b"fake data")
}

pub fn copy_file_to(source: &str, destination: &str) {
    std::fs::copy(source, destination).expect("can not copy file");
}

pub fn files_exist_at(files: Vec<impl AsRef<Path>>, path: impl AsRef<Path>) -> bool {
    files.iter().all(|f| {
        let mut loc = PathBuf::from(path.as_ref());
        loc.push(f);
        loc.exists()
    })
}

pub fn delete_directory_at(full_path: &str) {
    std::fs::remove_dir_all(PathBuf::from(full_path)).expect("can not remove directory");
}

pub fn executable_path() -> PathBuf {
    let mut path = binaries();
    path.push("nu");
    path
}

pub fn binaries() -> PathBuf {
    let mut path = PathBuf::new();
    path.push("target");
    path.push("debug");
    path
}

pub fn in_directory(str: impl AsRef<Path>) -> String {
    str.as_ref().display().to_string()
}
