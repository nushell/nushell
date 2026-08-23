use std::{
    fmt::Write,
    fs,
    hash::{BuildHasher, RandomState},
    io,
    ops::Deref,
    path::{Component, Path, PathBuf},
    sync::LazyLock,
};

use itertools::Itertools;
use rand::RngExt;

#[allow(unused, reason = "doesn't matter anymore")]
pub mod deprecated;

/// Random process ID used to add entropy to temp directory names.
static PROCESS_ID: LazyLock<u16> = LazyLock::new(|| rand::rng().random());

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
    /// # let playground = Playground::new("crate::tests::example::dir")?;
    /// let dir = playground.dir("abc/def")?;
    ///
    /// assert!(dir.is_dir());
    /// # playground.close()?;
    /// # Ok(())
    /// # }
    /// ```
    fn dir(&self, path: impl AsRef<Path>) -> Result<PathBuf> {
        let dir = self.path().join(normalize_playground_path(path.as_ref())?);
        if let Err(err) = fs::create_dir_all(&dir) {
            return Err(PlaygroundError {
                kind: PlaygroundErrorKind::CreateDir,
                path: dir,
                io_error_kind: err.kind(),
                message: err.to_string(),
            });
        }

        Ok(dir)
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
    /// # let playground = Playground::new("crate::tests::example::empty_file")?;
    /// let file = playground.empty_file("some/file.empty")?;
    ///
    /// assert!(file.is_file());
    /// # playground.close()?;
    /// # Ok(())
    /// # }
    /// ```
    fn empty_file(&self, path: impl AsRef<Path>) -> Result<PathBuf> {
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
    /// # let playground = Playground::new("crate::tests::example::file")?;
    /// let text_file = playground.file("some/file.txt", "abc")?;
    /// let bytes_file = playground.file("bytes.bin", [1, 2, 3])?;
    /// let indented_file = playground.file(
    ///     "indented.txt",
    ///     indoc! {"
    ///         abc
    ///         def
    ///     "},
    /// )?;
    ///
    /// assert_eq!(std::fs::read_to_string(text_file).unwrap(), "abc");
    /// assert_eq!(std::fs::read(bytes_file).unwrap(), vec![1, 2, 3]);
    /// assert_eq!(std::fs::read_to_string(indented_file).unwrap(), "abc\ndef\n");
    /// # playground.close()?;
    /// # Ok(())
    /// # }
    /// ```
    fn file(&self, path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> Result<PathBuf> {
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

        if let Err(err) = fs::write(&file, contents) {
            return Err(PlaygroundError {
                kind: PlaygroundErrorKind::WriteFile,
                path: file,
                io_error_kind: err.kind(),
                message: err.to_string(),
            });
        }

        Ok(file)
    }

    /// Create a readonly file with contents inside the playground.
    ///
    /// The path is joined to [`path`](Self::path). Any missing parent
    /// directories are created before the file is written.
    ///
    /// Paths with a leading root are treated as playground-relative, so
    /// `/some/file.txt` is handled the same way as `some/file.txt`.
    ///
    /// # Platform Notes
    ///
    /// This uses [`std::fs::Permissions::set_readonly`], whose exact behavior
    /// depends on the platform.
    ///
    /// # Example
    ///
    /// ```
    /// # use nu_test_support::prelude::*;
    /// #
    /// # fn main() -> Result {
    /// # let playground = Playground::new("crate::tests::example::readonly_file")?;
    /// let file = playground.readonly_file("readonly.txt", "contents")?;
    ///
    /// assert!(
    ///     std::fs::metadata(file)?.permissions().readonly()
    /// );
    /// # playground.close()?;
    /// # Ok(())
    /// # }
    /// ```
    fn readonly_file(&self, path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> Result<PathBuf> {
        let path = self.file(path, contents)?;
        let mut permissions = fs::metadata(&path)
            .map_err(|err| PlaygroundError {
                kind: PlaygroundErrorKind::Metadata,
                path: path.clone(),
                io_error_kind: err.kind(),
                message: err.to_string(),
            })?
            .permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&path, permissions).map_err(|err| PlaygroundError {
            kind: PlaygroundErrorKind::SetPermissions,
            path: path.clone(),
            io_error_kind: err.kind(),
            message: err.to_string(),
        })?;

        Ok(path)
    }

    /// Create a symlink inside the playground.
    ///
    /// Both paths are joined to [`path`](Self::path). Any missing parent
    /// directories for the link path are created before the symlink is written.
    ///
    /// Paths with a leading root are treated as playground-relative, so
    /// `/some/link` is handled the same way as `some/link`.
    ///
    /// # Platform Notes
    ///
    /// Windows requires the original path to exist so the helper can choose
    /// between file and directory symlinks. Creating symlinks on Windows can
    /// also require developer mode or elevated permissions.
    ///
    /// # Example
    ///
    /// ```
    /// # use nu_test_support::prelude::*;
    /// #
    /// # fn main() -> Result {
    /// # let playground = Playground::new("crate::tests::example::symlink")?;
    /// playground.file("original.txt", "contents")?;
    /// let link = playground.symlink("original.txt", "links/original.txt")?;
    ///
    /// assert!(link.is_symlink());
    /// # playground.close()?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(any(unix, windows))]
    fn symlink(&self, original: impl AsRef<Path>, link: impl AsRef<Path>) -> Result<PathBuf> {
        let original = self
            .path()
            .join(normalize_playground_path(original.as_ref())?);
        let link = self.path().join(normalize_playground_path(link.as_ref())?);

        if let Some(parent) = link.parent()
            && let Err(err) = fs::create_dir_all(parent)
        {
            return Err(PlaygroundError {
                kind: PlaygroundErrorKind::CreateDir,
                path: parent.into(),
                io_error_kind: err.kind(),
                message: err.to_string(),
            });
        }

        #[cfg(unix)]
        let symlink = std::os::unix::fs::symlink;

        #[cfg(windows)]
        let symlink = match original.metadata().map_err(|err| PlaygroundError {
            kind: PlaygroundErrorKind::Metadata,
            path: original.clone(),
            io_error_kind: err.kind(),
            message: err.to_string(),
        })? {
            metadata if metadata.is_dir() => std::os::windows::fs::symlink_dir,
            metadata if metadata.is_file() => std::os::windows::fs::symlink_file,
            _ => {
                return Err(PlaygroundError {
                    kind: PlaygroundErrorKind::InvalidSymlinkOriginal,
                    path: original.clone(),
                    io_error_kind: io::ErrorKind::Other,
                    message: String::new(),
                });
            }
        };

        symlink(original, &link).map_err(|err| PlaygroundError {
            kind: PlaygroundErrorKind::CreateSymlink,
            path: link.clone(),
            io_error_kind: err.kind(),
            message: err.to_string(),
        })?;

        Ok(link)
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
    /// # let playground = Playground::new("crate::tests::example::at")?;
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
    /// Create a playground for a generated test path.
    ///
    /// This constructor is primarily intended for code generated by the test macros.
    /// It expects a stable, fully-qualified test path, which is used to create a deterministic
    /// temporary directory name that is easy to identify when debugging leaked test directories.
    ///
    /// Most tests should not call this directly. 
    /// Prefer using the playground value injected by the test macro, then use the 
    /// [`PlaygroundFs`] methods to create files and directories inside it.
    // TODO: add an example showcasing how the macro would inject this
    #[doc(hidden)]
    #[allow(
        unstable_name_collisions,
        reason = "this is only testing code, rustc is fixed"
    )]
    pub fn new(test_path: impl AsRef<str>) -> Result<Self> {
        let test_path = test_path.as_ref();
        let mut dir_name = String::with_capacity(
            "nushell-testing-".len()
                + test_path.len()
                + 16 // max path hash
                + 4 // max process id hash
                + 2, // separators before the hash and process id
        );

        dir_name.push_str("nushell-testing-");
        test_path
            .split("::")
            .tail(3)
            .intersperse(".")
            .for_each(|segment| dir_name.push_str(segment));
        let _ = dir_name.write_fmt(format_args!(
            "-{:x}-{:x}",
            RANDOM_STATE.hash_one(test_path),
            PROCESS_ID.deref()
        ));

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

        #[cfg(windows)]
        if let Err(err) = clear_readonly_recursive(&self.temp_dir) {
            return Err(PlaygroundError {
                kind: PlaygroundErrorKind::Close,
                path: self.temp_dir.clone(),
                io_error_kind: err.kind(),
                message: err.to_string(),
            });
        }

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
            #[cfg(windows)]
            let _ = clear_readonly_recursive(&self.temp_dir);

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

    #[error("could not get metadata")]
    Metadata,

    #[error("could not set permissions")]
    SetPermissions,

    #[error("invalid original for symlink")]
    InvalidSymlinkOriginal,

    #[error("could not create symlink")]
    CreateSymlink,

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

#[cfg(windows)]
fn clear_readonly_recursive(path: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        return Ok(());
    }

    if file_type.is_dir() {
        for entry in fs::read_dir(path)? {
            clear_readonly_recursive(&entry?.path())?;
        }
    }

    let mut permissions = metadata.permissions();
    if permissions.readonly() {
        #[cfg_attr(windows, allow(clippy::permissions_set_readonly_false))]
        permissions.set_readonly(false);
        fs::set_permissions(path, permissions)?;
    }

    Ok(())
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

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn close_does_not_clear_readonly_permissions_through_directory_symlink()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let external = tempfile::tempdir()?;
        let external_file = external.path().join("external.txt");
        fs::write(&external_file, "contents")?;

        let mut permissions = fs::metadata(&external_file)?.permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&external_file, permissions)?;

        let playground = Playground::new(
            "crate::tests::playground::close_does_not_clear_readonly_permissions_through_directory_symlink",
        )?;
        std::os::windows::fs::symlink_dir(external.path(), playground.path().join("external"))?;

        playground.close()?;

        let readonly = fs::metadata(&external_file)?.permissions().readonly();

        let mut permissions = fs::metadata(&external_file)?.permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        permissions.set_readonly(false);
        fs::set_permissions(&external_file, permissions)?;

        assert!(readonly);
        Ok(())
    }
}
