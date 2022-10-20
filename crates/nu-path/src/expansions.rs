use std::io;
use std::path::{Path, PathBuf};

use super::dots::{expand_dots, expand_ndots};
use super::helpers;
use super::tilde::expand_tilde;

// Join a path relative to another path. Paths starting with tilde are considered as absolute.
fn join_path_relative<P, Q>(path: P, relative_to: Q) -> PathBuf
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
    } else if path.to_string_lossy().as_ref().starts_with('~') {
        // do not end up with "/some/path/~" or "/some/path/~user"
        path.into()
    } else {
        relative_to.join(path)
    }
}

fn canonicalize(path: impl AsRef<Path>) -> io::Result<PathBuf> {
    let path = expand_tilde(path);
    let path = expand_ndots(path);

    helpers::canonicalize(&path)
}

/// Resolve all symbolic links and all components (tilde, ., .., ...+) and return the path in its
/// absolute form.
///
/// Fails under the same conditions as
/// [std::fs::canonicalize](https://doc.rust-lang.org/std/fs/fn.canonicalize.html).
/// The input path is specified relative to another path
pub fn canonicalize_with<P, Q>(path: P, relative_to: Q) -> io::Result<PathBuf>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let path = join_path_relative(path, relative_to);

    canonicalize(path)
}

fn expand_path(path: impl AsRef<Path>) -> PathBuf {
    let path = expand_to_real_path(path);
    expand_dots(path)
}

/// Resolve only path components (tilde, ., .., ...+), if possible.
///
/// The function works in a "best effort" mode: It does not fail but rather returns the unexpanded
/// version if the expansion is not possible.
///
/// Furthermore, unlike canonicalize(), it does not use sys calls (such as readlink).
///
/// Does not convert to absolute form nor does it resolve symlinks.
/// The input path is specified relative to another path
pub fn expand_path_with<P, Q>(path: P, relative_to: Q) -> PathBuf
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let path = join_path_relative(path, relative_to);

    expand_path(path)
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
