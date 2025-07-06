#[cfg(windows)]
use omnipath::WinPathExt;
use std::io;
use std::path::{Path, PathBuf};

use super::dots::{expand_dots, expand_ndots};
use super::tilde::expand_tilde;

// Join a path relative to another path. Paths starting with tilde are considered as absolute.
fn join_path_relative<P, Q>(path: P, relative_to: Q, expand_tilde: bool) -> PathBuf
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let path = path.as_ref();
    let relative_to = relative_to.as_ref();

    if path == Path::new(".") {
        // Joining a Path with '.' appends a '.' at the end, making the prompt
        // more ugly - so we don't do anything, which should result in an equal
        // path on all supported systems.
        relative_to.into()
    } else if path.to_string_lossy().as_ref().starts_with('~') && expand_tilde {
        // do not end up with "/some/path/~" or "/some/path/~user"
        path.into()
    } else {
        relative_to.join(path)
    }
}

fn canonicalize(path: impl AsRef<Path>) -> io::Result<PathBuf> {
    let path = expand_tilde(path);
    let path = expand_ndots(path);
    canonicalize_path(&path)
}

#[cfg(windows)]
fn canonicalize_path(path: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
    path.canonicalize()?.to_winuser_path()
}

#[cfg(not(windows))]
fn canonicalize_path(path: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
    path.canonicalize()
}

/// Resolve all symbolic links and all components (tilde, ., .., ...+) and return the path in its
/// absolute form.
///
/// Fails under the same conditions as
/// [`std::fs::canonicalize`](https://doc.rust-lang.org/std/fs/fn.canonicalize.html).
/// The input path is specified relative to another path
pub fn canonicalize_with<P, Q>(path: P, relative_to: Q) -> io::Result<PathBuf>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let path = join_path_relative(path, relative_to, true);

    canonicalize(path)
}

/// Resolve only path components (tilde, ., .., ...+), if possible.
///
/// Doesn't convert to absolute form or use syscalls. Output may begin with "../"
pub fn expand_path(path: impl AsRef<Path>, need_expand_tilde: bool) -> PathBuf {
    let path = if need_expand_tilde {
        expand_tilde(path)
    } else {
        PathBuf::from(path.as_ref())
    };
    let path = expand_ndots(path);
    expand_dots(path)
}

/// Resolve only path components (tilde, ., .., ...+), if possible.
///
/// The function works in a "best effort" mode: It does not fail but rather returns the unexpanded
/// version if the expansion is not possible.
///
/// Furthermore, unlike canonicalize(), it does not use sys calls (such as readlink).
///
/// Converts to absolute form but does not resolve symlinks.
/// The input path is specified relative to another path
pub fn expand_path_with<P, Q>(path: P, relative_to: Q, expand_tilde: bool) -> PathBuf
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let path = join_path_relative(path, relative_to, expand_tilde);

    expand_path(path, expand_tilde)
}

/// Resolve to a path that is accepted by the system and no further - tilde is expanded, and ndot path components are expanded.
///
/// This function will take a leading tilde path component, and expand it to the user's home directory;
/// it will also expand any path elements consisting of only dots into the correct number of `..` path elements.
/// It does not do any normalization except to what will be accepted by Path::open,
/// and it does not touch the system at all, except for getting the home directory of the current user.
pub fn expand_to_real_path<P>(path: P) -> PathBuf
where
    P: AsRef<Path>,
{
    let path = expand_tilde(path);
    expand_ndots(path)
}

/// Attempts to canonicalize the path against the current directory. Failing that, if
/// the path is relative, it attempts all of the dirs in `dirs`. If that fails, it returns
/// the original error.
pub fn locate_in_dirs<I, P>(
    filename: impl AsRef<Path>,
    cwd: impl AsRef<Path>,
    dirs: impl FnOnce() -> I,
) -> std::io::Result<PathBuf>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let filename = filename.as_ref();
    let cwd = cwd.as_ref();
    match canonicalize_with(filename, cwd) {
        Ok(path) => Ok(path),
        Err(err) => {
            // Try to find it in `dirs` first, before giving up
            let mut found = None;
            for dir in dirs() {
                if let Ok(path) =
                    canonicalize_with(dir, cwd).and_then(|dir| canonicalize_with(filename, dir))
                {
                    found = Some(path);
                    break;
                }
            }
            found.ok_or(err)
        }
    }
}
