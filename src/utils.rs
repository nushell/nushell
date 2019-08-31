use crate::errors::ShellError;
use std::fmt;
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
        AbsolutePath::new(self.inner.parent().unwrap())
    }
}

impl From<AbsoluteFile> for PathBuf {
    fn from(file: AbsoluteFile) -> Self {
        file.inner
    }
}

impl fmt::Display for AbsoluteFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner.display())
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

impl fmt::Display for AbsolutePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner.display())
    }
}

impl Div<&str> for &AbsolutePath {
    type Output = AbsolutePath;

    fn div(self, rhs: &str) -> Self::Output {
        let parts = rhs.split("/");
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
        let parts = rhs.as_ref().split("/");
        let mut result = self.inner.clone();

        for part in parts {
            result = result.join(part);
        }

        RelativePath::new(result)
    }
}

impl fmt::Display for RelativePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner.display())
    }
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Res {
    pub loc: PathBuf,
    pub at: usize,
}

impl Res {}

pub struct FileStructure {
    root: PathBuf,
    pub resources: Vec<Res>,
}

impl FileStructure {
    pub fn new() -> FileStructure {
        FileStructure {
            root: PathBuf::new(),
            resources: Vec::<Res>::new(),
        }
    }

    pub fn contains_more_than_one_file(&self) -> bool {
        self.resources.len() > 1
    }

    pub fn contains_files(&self) -> bool {
        self.resources.len() > 0
    }

    pub fn set_root(&mut self, path: &Path) {
        self.root = path.to_path_buf();
    }

    pub fn paths_applying_with<F>(
        &mut self,
        to: F,
    ) -> Result<Vec<(PathBuf, PathBuf)>, Box<dyn std::error::Error>>
    where
        F: Fn((PathBuf, usize)) -> Result<(PathBuf, PathBuf), Box<dyn std::error::Error>>,
    {
        self.resources
            .iter()
            .map(|f| (PathBuf::from(&f.loc), f.at))
            .map(|f| to(f))
            .collect()
    }

    pub fn walk_decorate(&mut self, start_path: &Path) -> Result<(), ShellError> {
        self.set_root(&dunce::canonicalize(start_path)?);
        self.resources = Vec::<Res>::new();
        self.build(start_path, 0)?;
        self.resources.sort();

        Ok(())
    }

    fn build(&mut self, src: &Path, lvl: usize) -> Result<(), ShellError> {
        let source = dunce::canonicalize(src)?;

        if source.is_dir() {
            for entry in std::fs::read_dir(&source)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    self.build(&path, lvl + 1)?;
                }

                self.resources.push(Res {
                    loc: path.to_path_buf(),
                    at: lvl,
                });
            }
        } else {
            self.resources.push(Res {
                loc: source,
                at: lvl,
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::{FileStructure, Res};
    use std::path::PathBuf;

    fn fixtures() -> PathBuf {
        let mut sdx = PathBuf::new();
        sdx.push("tests");
        sdx.push("fixtures");
        sdx.push("formats");

        match dunce::canonicalize(sdx) {
            Ok(path) => path,
            Err(_) => panic!("Wrong path."),
        }
    }

    #[test]
    fn prepares_and_decorates_source_files_for_copying() {
        let mut res = FileStructure::new();

        res.walk_decorate(fixtures().as_path())
            .expect("Can not decorate files traversal.");

        assert_eq!(
            res.resources,
            vec![
                Res {
                    loc: fixtures().join("appveyor.yml"),
                    at: 0
                },
                Res {
                    loc: fixtures().join("caco3_plastics.csv"),
                    at: 0
                },
                Res {
                    loc: fixtures().join("caco3_plastics.tsv"),
                    at: 0
                },
                Res {
                    loc: fixtures().join("cargo_sample.toml"),
                    at: 0
                },
                Res {
                    loc: fixtures().join("jonathan.xml"),
                    at: 0
                },
                Res {
                    loc: fixtures().join("sample.bson"),
                    at: 0
                },
                Res {
                    loc: fixtures().join("sample.db"),
                    at: 0
                },
                Res {
                    loc: fixtures().join("sample.ini"),
                    at: 0
                },
                Res {
                    loc: fixtures().join("sgml_description.json"),
                    at: 0
                },
                Res {
                    loc: fixtures().join("utf16.ini"),
                    at: 0
                }
            ]
        );
    }
}
