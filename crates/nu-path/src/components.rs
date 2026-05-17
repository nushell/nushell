//! A wrapper around `Path::components()` that preserves trailing slashes.
//!
//! Trailing slashes are semantically important for us. For example, POSIX says
//! that path resolution should always follow the final symlink if it has
//! trailing slashes. Here's a demonstration:
//!
//! ```sh
//! mkdir foo
//! ln -s foo link
//!
//! cp -r link bar      # This copies the symlink, so bar is now a symlink to foo
//! cp -r link/ baz     # This copies the directory, so baz is now a directory
//! ```
//!
//! However, `Path::components()` normalizes trailing slashes away, and so does
//! other APIs that uses `Path::components()` under the hood, such as
//! `Path::parent()`. This is not ideal for path manipulation.
//!
//! This module provides a wrapper around `Path::components()` that produces an
//! empty component when there's a trailing slash.
//!
//! You can reconstruct a path with a trailing slash by concatenating the
//! components returned by this function using `PathBuf::push()` or
//! `Path::join()`. It works because `PathBuf::push("")` will add a trailing
//! slash when the original path doesn't have one.

use std::{
    ffi::OsStr,
    path::{Component, Path},
};

use crate::trailing_slash::has_trailing_slash;

/// Like `Path::components()`, but produces an extra empty component at the end
/// when `path` contains a trailing slash.
///
/// Example:
///
/// ```
/// # use std::path::{Path, Component};
/// # use std::ffi::OsStr;
/// use nu_path::components;
///
/// let path = Path::new("/foo/bar/");
/// let mut components = components(path);
///
/// assert_eq!(components.next(), Some(Component::RootDir));
/// assert_eq!(components.next(), Some(Component::Normal(OsStr::new("foo"))));
/// assert_eq!(components.next(), Some(Component::Normal(OsStr::new("bar"))));
/// assert_eq!(components.next(), Some(Component::Normal(OsStr::new(""))));
/// assert_eq!(components.next(), None);
/// ```
pub fn components(path: &Path) -> impl Iterator<Item = Component<'_>> {
    let mut final_component = Some(Component::Normal(OsStr::new("")));
    path.components().chain(std::iter::from_fn(move || {
        if has_trailing_slash(path) {
            final_component.take()
        } else {
            None
        }
    }))
}

#[cfg(test)]
mod test {
    //! We'll go through every variant of Component, with or without trailing
    //! slashes. Then we'll try reconstructing the path on some typical use cases.

    use crate::assert_path_eq;
    use std::{
        ffi::OsStr,
        path::{Component, Path, PathBuf},
    };

    #[test]
    fn empty_path() {
        let path = Path::new("");
        let mut components = crate::components(path);

        assert_eq!(components.next(), None);
    }

    #[test]
    #[cfg(windows)]
    fn prefix_only() {
        let path = Path::new("C:");
        let mut components = crate::components(path);

        assert!(matches!(components.next(), Some(Component::Prefix(_))));
        assert_eq!(components.next(), None);
    }

    #[test]
    #[cfg(windows)]
    fn prefix_with_trailing_slash() {
        let path = Path::new("C:\\");
        let mut components = crate::components(path);

        assert!(matches!(components.next(), Some(Component::Prefix(_))));
        assert!(matches!(components.next(), Some(Component::RootDir)));
        assert_eq!(components.next(), Some(Component::Normal(OsStr::new(""))));
        assert_eq!(components.next(), None);
    }

    #[test]
    fn root() {
        let path = Path::new("/");
        let mut components = crate::components(path);

        assert!(matches!(components.next(), Some(Component::RootDir)));
        assert_eq!(components.next(), Some(Component::Normal(OsStr::new(""))));
        assert_eq!(components.next(), None);
    }

    #[test]
    fn cur_dir_only() {
        let path = Path::new(".");
        let mut components = crate::components(path);

        assert!(matches!(components.next(), Some(Component::CurDir)));
        assert_eq!(components.next(), None);
    }

    #[test]
    fn cur_dir_with_trailing_slash() {
        let path = Path::new("./");
        let mut components = crate::components(path);

        assert!(matches!(components.next(), Some(Component::CurDir)));
        assert_eq!(components.next(), Some(Component::Normal(OsStr::new(""))));
        assert_eq!(components.next(), None);
    }

    #[test]
    fn parent_dir_only() {
        let path = Path::new("..");
        let mut components = crate::components(path);

        assert!(matches!(components.next(), Some(Component::ParentDir)));
        assert_eq!(components.next(), None);
    }

    #[test]
    fn parent_dir_with_trailing_slash() {
        let path = Path::new("../");
        let mut components = crate::components(path);

        assert!(matches!(components.next(), Some(Component::ParentDir)));
        assert_eq!(components.next(), Some(Component::Normal(OsStr::new(""))));
        assert_eq!(components.next(), None);
    }

    #[test]
    fn normal_only() {
        let path = Path::new("foo");
        let mut components = crate::components(path);

        assert_eq!(
            components.next(),
            Some(Component::Normal(OsStr::new("foo")))
        );
        assert_eq!(components.next(), None);
    }

    #[test]
    fn normal_with_trailing_slash() {
        let path = Path::new("foo/");
        let mut components = crate::components(path);

        assert_eq!(
            components.next(),
            Some(Component::Normal(OsStr::new("foo")))
        );
        assert_eq!(components.next(), Some(Component::Normal(OsStr::new(""))));
        assert_eq!(components.next(), None);
    }

    #[test]
    #[cfg(not(windows))]
    fn reconstruct_unix_only() {
        let path = Path::new("/home/Alice");

        let mut buf = PathBuf::new();
        for component in crate::components(path) {
            buf.push(component);
        }

        assert_path_eq!(path, buf);
    }

    #[test]
    #[cfg(not(windows))]
    fn reconstruct_unix_with_trailing_slash() {
        let path = Path::new("/home/Alice/");

        let mut buf = PathBuf::new();
        for component in crate::components(path) {
            buf.push(component);
        }

        assert_path_eq!(path, buf);
    }

    #[test]
    #[cfg(windows)]
    fn reconstruct_windows_only() {
        let path = Path::new("C:\\WINDOWS\\System32");

        let mut buf = PathBuf::new();
        for component in crate::components(path) {
            buf.push(component);
        }

        assert_path_eq!(path, buf);
    }

    #[test]
    #[cfg(windows)]
    fn reconstruct_windows_with_trailing_slash() {
        let path = Path::new("C:\\WINDOWS\\System32\\");

        let mut buf = PathBuf::new();
        for component in crate::components(path) {
            buf.push(component);
        }

        assert_path_eq!(path, buf);
    }
}
