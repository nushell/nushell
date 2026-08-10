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

/// Random process ID to have a very high entropy for temp dirs.
static PROCESS_ID: LazyLock<u16> = LazyLock::new(|| rand::rng().random());

/// Counter for playgrounds to ensure uniqueness even with same module path.
static PLAYGROUND_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Get the process temp dir once, shouldn't change over time.
static ENV_TEMP_DIR: LazyLock<PathBuf> = LazyLock::new(std::env::temp_dir);

/// [`RandomState`] for hashes that are comparable.
static RANDOM_STATE: LazyLock<RandomState> = LazyLock::new(RandomState::new);

type Result<T, E = PlaygroundError> = std::result::Result<T, E>;

mod sealed {
    pub trait Sealed {}
}

/// Filesystem operations for the [`Playground`].
pub trait PlaygroundFs: sealed::Sealed {
    /// [`Path`] to the current directory represented.
    fn path(&self) -> &Path;

    /// Create a directory inside the [`Playground`].
    ///
    /// Nested paths are allowed and will be joined to the current
    /// [`path`](Self::path) of the playground.
    /// All directories will be created passed to this method.
    /// Absolute paths are treated as relative to the playground, so
    /// `/abc/def` is equivalent to `abc/def`.
    ///
    /// # Example
    ///
    /// ```
    /// # use nu_test_support::prelude::*;
    /// #
    /// # fn main() -> Result {
    /// # let playground = Playground::new(module_path!())?;
    /// playground.dir("/abc/def")?;
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

    /// Create an empty file in the [`Playground`].
    ///
    /// The path will be joined to the current [`path`](Self::path) of the
    /// playground.
    /// Any parent directories will be added as necessary.
    /// Absolute paths are treated as relative to the playground, so
    /// `/some/file.empty` is equivalent to `some/file.empty`.
    ///
    /// # Example
    ///
    /// ```
    /// # use nu_test_support::prelude::*;
    /// #
    /// # fn main() -> Result {
    /// # let playground = Playground::new(module_path!())?;
    /// playground.empty_file("/some/file.empty")?;
    ///
    /// assert!(playground.path().join("some").join("file.empty").is_file());
    /// # playground.close()?;
    /// # Ok(())
    /// # }
    /// ```
    fn empty_file(&self, path: impl AsRef<Path>) -> Result<&Self> {
        self.file(normalize_playground_path(path.as_ref())?, [])
    }

    /// Create a file with contents in the [`Playground`].
    ///
    /// The path will be joined to the current [`path`](Self::path) of the
    /// playground.
    /// Any parent directories will be added as necessary.
    /// Absolute paths are treated as relative to the playground, so
    /// `/some/file.txt` is equivalent to `some/file.txt`.
    /// # Example
    ///
    /// ```
    /// # use nu_test_support::prelude::*;
    /// #
    /// # fn main() -> Result {
    /// # let playground = Playground::new(module_path!())?;
    /// playground.file("/some/file.txt", "abc")?;
    ///
    /// assert_eq!(
    ///     std::fs::read_to_string(playground.path().join("some").join("file.txt")).unwrap(),
    ///     "abc"
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

    /// At-API of the [`Playground`].
    ///
    /// This function allows nesting into directories without naming them
    /// repeatedly.
    /// The passed path will be joined to the [`path`](Self::path) of the
    /// playground and created as missing.
    /// Absolute paths are treated as relative to the playground, so
    /// `/abc` is equivalent to `abc`.
    ///
    /// # Example
    ///
    /// ```
    /// # use nu_test_support::prelude::*;
    /// #
    /// # fn main() -> Result {
    /// # let playground = Playground::new(module_path!())?;
    /// playground.at("abc", |at| {
    ///     at.empty_file("file0.empty")?;
    ///     at.empty_file("file1.empty")?;
    ///     at.at("def", |at| {
    ///         at.empty_file("file2.empty")?;
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

        match fs::create_dir_all(&path) {
            Ok(()) => {}
            Err(err) => {
                return Err(PlaygroundError {
                    kind: PlaygroundErrorKind::CreateDir,
                    path,
                    io_error_kind: err.kind(),
                    message: err.to_string(),
                }
                .into());
            }
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
    /// Automatically try to remove temp dir.
    ///
    /// Prefer [`close`](Playground::close) to explicitly remove the temp dir
    /// and get a [`Result`].
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
    let err = |kind| {
        Err(PlaygroundError {
            kind: PlaygroundErrorKind::InvalidPath(kind),
            path: path.into(),
            io_error_kind: io::ErrorKind::Other,
            message: "".to_string(),
        })
    };

    if path.as_os_str().is_empty() {
        return err(InvalidPlaygroundPath::Empty);
    }

    let mut valid_path = path;
    for (i, component) in valid_path.components().enumerate() {
        match (i, component) {
            (0, Component::RootDir) => {
                let mut components = valid_path.components();
                components.next();
                valid_path = components.as_path();
            }
            (_, Component::Prefix(_)) => return err(InvalidPlaygroundPath::IncludesPrefix),
            (_, Component::RootDir) => return err(InvalidPlaygroundPath::NestedRoot),
            (_, Component::ParentDir) => return err(InvalidPlaygroundPath::IncludesParentDir),
            (_, Component::Normal(_) | Component::CurDir) => (),
        }
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
