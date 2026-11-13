#[cfg(windows)]
use omnipath::WinPathExt;
use std::borrow::Cow;
use std::io;
use std::path::{Path, PathBuf, absolute};

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
/// Consider using [`absolute_with`] if you don't need to resolve symlinks.
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

/// Returns the path in its absolute form without resolving symlinks.
///
/// On Unix systems, .. components are a kind of link so these aren't resolved.
/// On Windows .. components are resoloved.
///
/// Fails under the same conditions as
/// [`std::path::absolute`](https://doc.rust-lang.org/std/path/fn.absolute.html).
/// In practice this is only likely to fail if both paths are empty.
pub fn absolute_with<P, Q>(path: P, relative_to: Q) -> io::Result<PathBuf>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let path = join_path_relative(path, relative_to, true);
    let path = expand_tilde(path);
    let path = expand_ndots(path);
    absolute(path)
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

/// Resolves a Windows drive-relative path such as `d:` or `d:foo` the way Windows does.
///
/// On the drive of `cwd`, the path is relative to `cwd`. On another drive, it is relative to
/// that drive's current directory as Windows tracks it (the `=D:` environment variable),
/// which falls back to the drive's root. Any other path, and every path on other platforms,
/// is returned unchanged without allocating.
pub fn resolve_drive_relative<'a>(path: &'a Path, cwd: &Path) -> Cow<'a, Path> {
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};

        fn disk(path: &Path) -> Option<u8> {
            match path.components().next() {
                Some(Component::Prefix(prefix)) => match prefix.kind() {
                    Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => Some(letter),
                    _ => None,
                },
                _ => None,
            }
        }

        let Some(letter) = disk(path).filter(|_| !path.has_root()) else {
            return Cow::Borrowed(path);
        };
        let mut components = path.components();
        components.next();
        let rest = components.as_path();

        if disk(cwd).is_some_and(|cwd_letter| cwd_letter.eq_ignore_ascii_case(&letter)) {
            return if rest.as_os_str().is_empty() {
                Cow::Owned(cwd.to_path_buf())
            } else {
                Cow::Owned(cwd.join(rest))
            };
        }
        match absolute(expand_ndots(path)) {
            Ok(resolved) => Cow::Owned(resolved),
            Err(_) => Cow::Borrowed(path),
        }
    }
    #[cfg(not(windows))]
    {
        let _ = cwd;
        Cow::Borrowed(path)
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn drive_relative_on_current_drive_uses_cwd() {
        let cwd = Path::new(r"C:\Users\me");
        assert_eq!(resolve_drive_relative(Path::new("c:"), cwd), cwd);
        assert_eq!(resolve_drive_relative(Path::new("C:"), cwd), cwd);
        assert_eq!(
            resolve_drive_relative(Path::new("c:src"), cwd),
            Path::new(r"C:\Users\me\src")
        );
        assert_eq!(
            resolve_drive_relative(Path::new("c:.."), cwd),
            Path::new(r"C:\Users\me\..")
        );
    }

    #[test]
    fn drive_relative_on_other_drive_is_absolute() {
        let resolved = resolve_drive_relative(Path::new("q:"), Path::new(r"C:\Users\me"));
        assert!(resolved.is_absolute(), "{resolved:?}");
        assert!(resolved.starts_with(r"Q:\"), "{resolved:?}");
    }

    #[test]
    fn other_paths_are_borrowed() {
        let cwd = Path::new(r"C:\Users\me");
        for path in [r"d:\", r"C:\Windows", "foo", r"\server\share", "~"] {
            assert!(matches!(
                resolve_drive_relative(Path::new(path), cwd),
                Cow::Borrowed(_)
            ));
        }
    }
}
