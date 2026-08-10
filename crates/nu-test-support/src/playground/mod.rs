use std::{
    fs,
    hash::{BuildHasher, RandomState},
    io,
    ops::Deref,
    path::{Component, Path, PathBuf},
    sync::{
        LazyLock,
        atomic::{AtomicU64, Ordering},
    },
};

use rand::RngExt;

#[allow(unused, reason = "doesn't matter anymore")]
pub mod deprecated;

/// Random process ID used to add entropy to temp directory names.
static PROCESS_ID: LazyLock<u16> = LazyLock::new(|| rand::rng().random());

/// Global counter that keeps playground names unique, even for the same module path.
static PLAYGROUND_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Process temp directory, captured once for the lifetime of the process.
static ENV_TEMP_DIR: LazyLock<PathBuf> = LazyLock::new(std::env::temp_dir);

/// [`RandomState`] used for stable hashes within this process.
static RANDOM_STATE: LazyLock<RandomState> = LazyLock::new(RandomState::new);

type Result<T, E = PlaygroundError> = std::result::Result<T, E>;

mod sealed {
    pub trait Sealed {}
}

/// Filesystem operations for playground paths.
pub trait PlaygroundFs: sealed::Sealed {
    /// Path represented by this playground handle.
    fn path(&self) -> &Path;

    /// Create a directory inside the playground.
    ///
    /// The path is joined to [`path`](Self::path). Nested paths are allowed,
    /// and any missing directories in the path are created.
    ///
    /// Paths with a leading root are treated as playground-relative, so
    /// `/abc/def` is handled the same way as `abc/def`.
    ///
    /// # Example
    ///
    /// ```
    /// # use nu_test_support::prelude::*;
    /// #
    /// # fn main() -> Result {
    /// # let playground = Playground::new(module_path!())?;
    /// playground.dir("abc/def")?;
    ///
    /// assert!(playground.path().join("abc").join("def").is_dir());
    /// # playground.close()?;
    /// # Ok(())
    /// # }
    /// ```
    fn dir(&self, path: impl AsRef<Path>) -> Result<&Self> {
        let dir = self.path().join(normalize_playground_path(path.as_ref())?);
        fs::create_dir_all(&dir)
            .map(|()| self)
            .map_err(|err| PlaygroundError {
                kind: PlaygroundErrorKind::CreateDir,
                path: dir,
                io_error_kind: err.kind(),
                message: err.to_string(),
            })
    }

    /// Create an empty file inside the playground.
    ///
    /// The path is joined to [`path`](Self::path). Any missing parent
    /// directories are created before the file is written.
    ///
    /// Paths with a leading root are treated as playground-relative, so
    /// `/some/file.empty` is handled the same way as `some/file.empty`.
    ///
    /// # Example
    ///
    /// ```
    /// # use nu_test_support::prelude::*;
    /// #
    /// # fn main() -> Result {
    /// # let playground = Playground::new(module_path!())?;
    /// playground.empty_file("some/file.empty")?;
    ///
    /// assert!(playground.path().join("some").join("file.empty").is_file());
    /// # playground.close()?;
    /// # Ok(())
    /// # }
    /// ```
    fn empty_file(&self, path: impl AsRef<Path>) -> Result<&Self> {
        self.file(path, [])
    }

    /// Create a file with contents inside the playground.
    ///
    /// The path is joined to [`path`](Self::path). Any missing parent
    /// directories are created before the file is written.
    ///
    /// Paths with a leading root are treated as playground-relative, so
    /// `/some/file.txt` is handled the same way as `some/file.txt`.
    ///
    /// # Example
    ///
    /// ```
    /// # use indoc::indoc;
    /// # use nu_test_support::prelude::*;
    /// #
    /// # fn main() -> Result {
    /// # let playground = Playground::new(module_path!())?;
    /// playground.file("some/file.txt", "abc")?;
    /// playground.file("bytes.bin", [1, 2, 3])?;
    /// playground.file(
    ///     "indented.txt",
    ///     indoc! {"
    ///         abc
    ///         def
    ///     "},
    /// )?;
    ///
    /// assert_eq!(
    ///     std::fs::read_to_string(playground.path().join("some").join("file.txt")).unwrap(),
    ///     "abc"
    /// );
    /// assert_eq!(
    ///     std::fs::read(playground.path().join("bytes.bin")).unwrap(),
    ///     vec![1, 2, 3]
    /// );
    /// assert_eq!(
    ///     std::fs::read_to_string(playground.path().join("indented.txt")).unwrap(),
    ///     "abc\ndef\n"
    /// );
    /// # playground.close()?;
    /// # Ok(())
    /// # }
    /// ```
    fn file(&self, path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> Result<&Self> {
        let file = self.path().join(normalize_playground_path(path.as_ref())?);
        if let Some(parent) = file.parent()
            && let Err(err) = fs::create_dir_all(parent)
        {
            return Err(PlaygroundError {
                kind: PlaygroundErrorKind::CreateDir,
                path: parent.into(),
                io_error_kind: err.kind(),
                message: err.to_string(),
            });
        }

        fs::write(&file, contents)
            .map(|()| self)
            .map_err(|err| PlaygroundError {
                kind: PlaygroundErrorKind::WriteFile,
                path: file,
                io_error_kind: err.kind(),
                message: err.to_string(),
            })
    }

    /// Create a nested playground directory and run filesystem operations inside it.
    ///
    /// The path is joined to [`path`](Self::path), and the directory is
    /// created before the closure runs. Use this to group setup for several
    /// files under the same directory without repeating the directory name.
    ///
    /// Paths with a leading root are treated as playground-relative, so
    /// `/abc` is handled the same way as `abc`.
    ///
    /// # Example
    ///
    /// ```
    /// # use nu_test_support::prelude::*;
    /// #
    /// # fn main() -> Result {
    /// # let playground = Playground::new(module_path!())?;
    /// playground.at("abc", |dir| {
    ///     dir.empty_file("file0.empty")?;
    ///     dir.empty_file("file1.empty")?;
    ///     dir.at("def", |nested| {
    ///         nested.empty_file("file2.empty")?;
    ///         Ok(())
    ///     })?;
    ///     Ok(())
    /// })?;
    ///
    /// assert!(playground.path().join("abc/file0.empty").is_file());
    /// assert!(playground.path().join("abc/file1.empty").is_file());
    /// assert!(playground.path().join("abc/def/file2.empty").is_file());
    /// # playground.close()?;
    /// # Ok(())
    /// # }
    /// ```
    fn at<R>(
        &self,
        path: impl AsRef<Path>,
        inside: impl FnOnce(&PlaygroundAt) -> Result<R>,
    ) -> Result<R> {
        let path = self.path().join(normalize_playground_path(path.as_ref())?);

        if let Err(err) = fs::create_dir_all(&path) {
            return Err(PlaygroundError {
                kind: PlaygroundErrorKind::CreateDir,
                path,
                io_error_kind: err.kind(),
                message: err.to_string(),
            });
        }

        let at = PlaygroundAt { path };
        inside(&at)
    }
}

pub struct Playground {
    temp_dir: PathBuf,
    closed: bool,
}

// compatibility
impl Playground {
    pub fn setup<R>(
        topic: &str,
        block: impl FnOnce(deprecated::Dirs, &mut deprecated::Playground) -> R,
    ) -> R {
        deprecated::Playground::setup(topic, block)
    }
}

impl Playground {
    pub fn new(module_path: impl AsRef<str>) -> Result<Self> {
        let module_path_hash = RANDOM_STATE.hash_one(module_path.as_ref());
        let dir_name = format!(
            "nushell-testing-{module_path_hash:x}-{process_id:x}-{playground_counter:x}",
            process_id = PROCESS_ID.deref(),
            playground_counter = PLAYGROUND_COUNTER.fetch_add(1, Ordering::Relaxed)
        );

        let temp_dir = ENV_TEMP_DIR.join(dir_name);
        if let Err(err) = fs::create_dir(&temp_dir) {
            return Err(PlaygroundError {
                kind: PlaygroundErrorKind::Open,
                path: temp_dir,
                io_error_kind: err.kind(),
                message: err.to_string(),
            });
        }

        Ok(Self {
            temp_dir,
            closed: false,
        })
    }

    #[track_caller]
    pub fn close(mut self) -> Result<()> {
        assert!(!self.closed, "playground already closed");
        fs::remove_dir_all(&self.temp_dir)
            .inspect(|()| self.closed = true)
            .map_err(|err| PlaygroundError {
                kind: PlaygroundErrorKind::Close,
                path: self.temp_dir.clone(),
                io_error_kind: err.kind(),
                message: err.to_string(),
            })
    }
}

impl sealed::Sealed for Playground {}
impl PlaygroundFs for Playground {
    fn path(&self) -> &Path {
        self.temp_dir.as_path()
    }
}

impl Drop for Playground {
    /// Try to remove the temp directory automatically.
    ///
    /// Prefer [`close`](Playground::close) when cleanup errors should be
    /// reported to the test.
    fn drop(&mut self) {
        if !self.closed {
            let _ = fs::remove_dir_all(&self.temp_dir);
        }
    }
}

pub struct PlaygroundAt {
    path: PathBuf,
}

impl sealed::Sealed for PlaygroundAt {}
impl PlaygroundFs for PlaygroundAt {
    fn path(&self) -> &Path {
        self.path.as_path()
    }
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("{kind} at {path}, {message} ({io_error_kind})")]
pub struct PlaygroundError {
    kind: PlaygroundErrorKind,
    path: PathBuf,
    io_error_kind: io::ErrorKind,
    message: String,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum PlaygroundErrorKind {
    #[error(transparent)]
    InvalidPath(InvalidPlaygroundPath),

    #[error("could not open playground")]
    Open,

    #[error("could not create dir in playground")]
    CreateDir,

    #[error("could not write file in playground")]
    WriteFile,

    #[error("could not close playground")]
    Close,
}

fn normalize_playground_path(path: &Path) -> Result<&Path, PlaygroundError> {
    let err = |kind| PlaygroundError {
        kind: PlaygroundErrorKind::InvalidPath(kind),
        path: path.into(),
        io_error_kind: io::ErrorKind::Other,
        message: String::new(),
    };

    let check_component = |component| match component {
        Component::Prefix(_) => Err(InvalidPlaygroundPath::IncludesPrefix),
        Component::RootDir => Err(InvalidPlaygroundPath::NestedRoot),
        Component::ParentDir => Err(InvalidPlaygroundPath::IncludesParentDir),
        Component::Normal(_) | Component::CurDir => Ok(()),
    };

    let mut valid_path = path;
    let mut components = valid_path.components();
    match components.next() {
        Some(Component::RootDir) => valid_path = components.as_path(),
        Some(c) => check_component(c).map_err(err)?,
        None => (),
    };
    components.try_for_each(check_component).map_err(err)?;

    if valid_path.as_os_str().is_empty() {
        return Err(err(InvalidPlaygroundPath::Empty));
    }

    Ok(valid_path)
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum InvalidPlaygroundPath {
    #[error("path is empty")]
    Empty,

    #[error("path includes prefix")]
    IncludesPrefix,

    #[error("path includes nested root")]
    NestedRoot,

    #[error("path includes parent dir")]
    IncludesParentDir,
}
