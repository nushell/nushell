use super::helpers;
use std::path::{Component, Path, PathBuf};

/// Normalize the path, expanding occurrences of n-dots.
///
/// It performs the same normalization as `nu_path::components()`, except it also expands n-dots,
/// such as "..." and "....", into multiple "..".
///
/// The resulting path will use platform-specific path separators, regardless of what path separators was used in the input.
pub fn expand_ndots(path: impl AsRef<Path>) -> PathBuf {
    // Returns whether a path component is n-dots.
    fn is_ndots(s: &std::ffi::OsStr) -> bool {
        s.as_encoded_bytes().iter().all(|c| *c == b'.') && s.len() >= 3
    }

    let path = path.as_ref();

    let mut result = PathBuf::with_capacity(path.as_os_str().len());
    for component in crate::components(path) {
        match component {
            Component::Normal(s) if is_ndots(s) => {
                let n = s.len();
                // Push ".." to the path (n - 1) times.
                for _ in 0..n - 1 {
                    result.push("..");
                }
            }
            _ => result.push(component),
        }
    }

    result
}

/// Normalize the path, expanding occurrences of "." and "..".
///
/// It performs the same normalization as `nu_path::components()`, except it also expands ".."
/// when its preceding component is a normal component, ignoring the possibility of symlinks.
/// In other words, it operates on the lexical structure of the path.
///
/// This won't expand "/.." even though the parent directory of "/" is often
/// considered to be itself.
///
/// The resulting path will use platform-specific path separators, regardless of what path separators was used in the input.
pub fn expand_dots(path: impl AsRef<Path>) -> PathBuf {
    // Check if the last component of the path is a normal component.
    fn last_component_is_normal(path: &Path) -> bool {
        matches!(path.components().last(), Some(Component::Normal(_)))
    }

    let path = path.as_ref();

    let mut result = PathBuf::with_capacity(path.as_os_str().len());
    for component in crate::components(path) {
        match component {
            Component::ParentDir if last_component_is_normal(&result) => {
                result.pop();
            }
            Component::CurDir if last_component_is_normal(&result) => {
                // no-op
            }
            _ => result.push(component),
        }
    }

    helpers::simiplified(&result)
}

#[cfg(test)]
mod test_expand_ndots {
    use super::*;
    use crate::assert_path_eq;

    #[test]
    fn empty_path() {
        let path = Path::new("");
        assert_path_eq!(expand_ndots(path), "");
    }

    #[test]
    fn root_dir() {
        let path = Path::new("/");
        let expected = if cfg!(windows) { "\\" } else { "/" };
        assert_path_eq!(expand_ndots(path), expected);
    }

    #[test]
    fn two_dots() {
        let path = Path::new("..");
        assert_path_eq!(expand_ndots(path), "..");
    }

    #[test]
    fn three_dots() {
        let path = Path::new("...");
        let expected = if cfg!(windows) { r"..\.." } else { "../.." };
        assert_path_eq!(expand_ndots(path), expected);
    }

    #[test]
    fn five_dots() {
        let path = Path::new(".....");
        let expected = if cfg!(windows) {
            r"..\..\..\.."
        } else {
            "../../../.."
        };
        assert_path_eq!(expand_ndots(path), expected);
    }

    #[test]
    fn three_dots_with_trailing_slash() {
        let path = Path::new("/tmp/.../");
        let expected = if cfg!(windows) {
            r"\tmp\..\..\"
        } else {
            "/tmp/../../"
        };
        assert_path_eq!(expand_ndots(path), expected);
    }

    #[test]
    fn filenames_with_dots() {
        let path = Path::new("...foo.../");
        let expected = if cfg!(windows) {
            r"...foo...\"
        } else {
            "...foo.../"
        };
        assert_path_eq!(expand_ndots(path), expected);
    }

    #[test]
    fn multiple_ndots() {
        let path = Path::new("..././...");
        let expected = if cfg!(windows) {
            r"..\..\..\.."
        } else {
            "../../../.."
        };
        assert_path_eq!(expand_ndots(path), expected);
    }
}

#[cfg(test)]
mod test_expand_dots {
    use super::*;
    use crate::assert_path_eq;

    #[test]
    fn empty_path() {
        let path = Path::new("");
        assert_path_eq!(expand_dots(path), "");
    }

    #[test]
    fn single_dot() {
        let path = Path::new("./");
        let expected = if cfg!(windows) { r".\" } else { "./" };
        assert_path_eq!(expand_dots(path), expected);
    }

    #[test]
    fn more_single_dots() {
        let path = Path::new("././.");
        let expected = ".";
        assert_path_eq!(expand_dots(path), expected);
    }

    #[test]
    fn double_dots() {
        let path = Path::new("../../..");
        let expected = if cfg!(windows) {
            r"..\..\.."
        } else {
            "../../.."
        };
        assert_path_eq!(expand_dots(path), expected);
    }

    #[test]
    fn backtrack_once() {
        let path = Path::new("/foo/bar/../baz/");
        let expected = if cfg!(windows) {
            r"\foo\baz\"
        } else {
            "/foo/baz/"
        };
        assert_path_eq!(expand_dots(path), expected);
    }

    #[test]
    fn backtrack_to_root() {
        let path = Path::new("/foo/bar/../../../../baz");
        let expected = if cfg!(windows) {
            r"\..\..\baz"
        } else {
            "/../../baz"
        };
        assert_path_eq!(expand_dots(path), expected);
    }
}
