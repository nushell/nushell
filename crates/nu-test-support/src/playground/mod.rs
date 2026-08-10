use std::{
    fs,
    hash::{BuildHasher, RandomState},
    io,
    marker::PhantomData,
    ops::Deref,
    path::{Path, PathBuf},
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
static ENV_TEMP_DIR: LazyLock<PathBuf> = LazyLock::new(|| std::env::temp_dir());

/// [`RandomState`] for hashes that are comparable.
static RANDOM_STATE: LazyLock<RandomState> = LazyLock::new(|| RandomState::new());

type Result<T, E = PlaygroundError> = std::result::Result<T, E>;

mod sealed {
    pub trait Sealed {}
}

pub trait PlaygroundFs: sealed::Sealed {
    fn path(&self) -> &Path;

    #[track_caller]
    fn dir(&self, path: impl AsRef<Path>) -> Result<&Self> {
        let dir = self.path().join(path);
        fs::create_dir_all(&dir)
            .map(|()| self)
            .map_err(|err| PlaygroundError {
                io_error: err,
                path: dir,
                kind: PlaygroundErrorKind::CreateDir,
            })
    }

    #[track_caller]
    fn empty_file(&self, path: impl AsRef<Path>) -> Result<&Self> {
        self.file(path, "")
    }

    #[track_caller]
    fn file(&self, path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> Result<&Self> {
        let file = self.path().join(path);
        if let Some(parent) = file.parent() {
            if let Err(err) = fs::create_dir_all(parent) {
                return Err(PlaygroundError {
                    io_error: err,
                    path: parent.into(),
                    kind: PlaygroundErrorKind::CreateDir,
                });
            }
        }

        fs::write(&file, contents)
            .map(|()| self)
            .map_err(|err| PlaygroundError {
                io_error: err,
                path: file,
                kind: PlaygroundErrorKind::WriteFile,
            })
    }

    fn at<'p, F, R>(&'p self, path: impl AsRef<Path>, inside: F) -> R
    where
        F: FnOnce(PlaygroundAt<'p>) -> R,
    {
        let at = PlaygroundAt {
            path: self.path().join(path),
            lifetime: PhantomData,
        };
        inside(at)
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
    #[track_caller]
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
                io_error: err,
                path: temp_dir,
                kind: PlaygroundErrorKind::Open,
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
                io_error: err,
                path: self.temp_dir.clone(),
                kind: PlaygroundErrorKind::Close,
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

pub struct PlaygroundAt<'p> {
    path: PathBuf,
    lifetime: PhantomData<&'p Playground>,
}

impl<'p> sealed::Sealed for PlaygroundAt<'p> {}
impl<'p> PlaygroundFs for PlaygroundAt<'p> {
    fn path(&self) -> &Path {
        self.path.as_path()
    }
}

#[expect(dead_code, reason = "only used for Debug impl")]
#[derive(Debug)]
pub struct PlaygroundError {
    kind: PlaygroundErrorKind,
    path: PathBuf,
    io_error: io::Error,
}

#[derive(Debug)]
pub enum PlaygroundErrorKind {
    Open,
    CreateDir,
    WriteFile,
    Close,
    AlreadyClosed,
}
