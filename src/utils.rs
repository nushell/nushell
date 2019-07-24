use std::ops::Div;
use std::path::{Path, PathBuf};

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

impl Div<&str> for AbsolutePath {
    type Output = AbsolutePath;

    fn div(self, rhs: &str) -> Self::Output {
        AbsolutePath {
            inner: self.inner.join(rhs),
        }
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

impl<T: AsRef<str>> Div<T> for RelativePath {
    type Output = RelativePath;

    fn div(self, rhs: T) -> Self::Output {
        RelativePath {
            inner: self.inner.join(rhs.as_ref()),
        }
    }
}
