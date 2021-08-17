use std::io;
use std::path::{Path, PathBuf};

use super::dots::{expand_dots, expand_ndots};
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
    } else if path.starts_with("~") {
        // do not end up with "/some/path/~"
        path.into()
    } else {
        relative_to.join(path)
    }
}

/// Resolve all symbolic links and all components (tilde, ., .., ...+) and return the path in its
/// absolute form.
///
/// Fails under the same conditions as std::fs::canonicalize().
pub fn canonicalize(path: impl AsRef<Path>) -> io::Result<PathBuf> {
    let path = expand_tilde(path);
    let path = expand_ndots(path);

    dunce::canonicalize(path)
}

/// Same as canonicalize() but the input path is composed of two parts
pub fn canonicalize_with<P, Q>(path: P, relative_to: Q) -> io::Result<PathBuf>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let path = join_path_relative(path, relative_to);

    canonicalize(path)
}

/// Resolve path components (tilde, ., .., ...+) if possible.
///
/// The function works in a "best effort" mode: It does not fail but rather returns the unexpanded
/// version if the expansion is not possible.
///
/// Furthermore, unlike canonicalize(), it does not use sys calls (such as readlink).
///
/// Does not convert to absolute form nor does it resolve symlinks.
pub fn expand_path(path: impl AsRef<Path>) -> PathBuf {
    let path = expand_tilde(path);
    let path = expand_ndots(path);
    expand_dots(path)
}

/// Same as expand_path() but the input path is composed of two parts
pub fn expand_path_with<P, Q>(path: P, relative_to: Q) -> PathBuf
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let path = join_path_relative(path, relative_to);

    expand_path(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_path_with_and_without_relative() {
        let relative_to = Path::new("/foo/bar");
        let path = Path::new("../..");
        let full_path = Path::new("/foo/bar/../..");

        assert_eq!(expand_path(full_path), expand_path_with(path, relative_to),);
    }

    #[test]
    fn expand_path_with_relative() {
        let relative_to = Path::new("/foo/bar");
        let path = Path::new("../..");

        assert_eq!(PathBuf::from("/"), expand_path_with(path, relative_to),);
    }

    #[test]
    fn canonicalize_should_fail() {
        let relative_to = Path::new("/foo/bar/baz"); // '/foo' is (hopefully) missing
        let path = Path::new("../..");

        assert!(canonicalize_with(path, relative_to).is_err());
    }
}
