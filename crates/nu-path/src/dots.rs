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
mod tests {
    use super::*;

    #[test]
    fn expand_two_dots() {
        let path = Path::new("/foo/bar/..");

        assert_eq!(
            PathBuf::from("/foo"), // missing path
            expand_dots(path)
        );
    }

    #[test]
    fn expand_dots_with_curdir() {
        let path = Path::new("/foo/./bar/./baz");

        assert_eq!(PathBuf::from("/foo/bar/baz"), expand_dots(path));
    }

    // track_caller refers, in the panic-message, to the line of the function call and not
    // inside of the function, which is nice for a test-helper-function
    #[track_caller]
    fn check_ndots_expansion(expected: &str, s: &str) {
        let expanded = expand_ndots(Path::new(s));
        assert_eq!(Path::new(expected), &expanded);
    }

    // common tests
    #[test]
    fn string_without_ndots() {
        check_ndots_expansion("../hola", "../hola");
    }

    #[test]
    fn string_with_three_ndots_and_chars() {
        check_ndots_expansion("a...b", "a...b");
    }

    #[test]
    fn string_with_two_ndots_and_chars() {
        check_ndots_expansion("a..b", "a..b");
    }

    #[test]
    fn string_with_one_dot_and_chars() {
        check_ndots_expansion("a.b", "a.b");
    }

    #[test]
    fn string_starts_with_dots() {
        check_ndots_expansion(".file", ".file");
        check_ndots_expansion("..file", "..file");
        check_ndots_expansion("...file", "...file");
        check_ndots_expansion("....file", "....file");
        check_ndots_expansion(".....file", ".....file");
    }

    #[test]
    fn string_ends_with_dots() {
        check_ndots_expansion("file.", "file.");
        check_ndots_expansion("file..", "file..");
        check_ndots_expansion("file...", "file...");
        check_ndots_expansion("file....", "file....");
        check_ndots_expansion("file.....", "file.....");
    }

    #[test]
    fn string_starts_and_ends_with_dots() {
        check_ndots_expansion(".file.", ".file.");
        check_ndots_expansion("..file..", "..file..");
        check_ndots_expansion("...file...", "...file...");
        check_ndots_expansion("....file....", "....file....");
        check_ndots_expansion(".....file.....", ".....file.....");
    }
    #[test]
    fn expand_multiple_dots() {
        check_ndots_expansion("../..", "...");
        check_ndots_expansion("../../..", "....");
        check_ndots_expansion("../../../..", ".....");
        check_ndots_expansion("../../../../", ".../...");
        check_ndots_expansion("../../file name/../../", ".../file name/...");
        check_ndots_expansion("../../../file name/../../../", "..../file name/....");
    }

    #[test]
    fn expand_dots_double_dots_no_change() {
        // Can't resolve this as we don't know our parent dir
        assert_eq!(Path::new(".."), expand_dots(Path::new("..")));
    }

    #[test]
    fn expand_dots_single_dot_no_change() {
        // Can't resolve this as we don't know our current dir
        assert_eq!(Path::new("."), expand_dots(Path::new(".")));
    }

    #[test]
    fn expand_dots_multi_single_dots_no_change() {
        assert_eq!(Path::new("././."), expand_dots(Path::new("././.")));
    }

    #[test]
    fn expand_multi_double_dots_no_change() {
        assert_eq!(Path::new("../../../"), expand_dots(Path::new("../../../")));
    }

    #[test]
    fn expand_dots_no_change_with_dirs() {
        // Can't resolve this as we don't know our parent dir
        assert_eq!(
            Path::new("../../../dir1/dir2/"),
            expand_dots(Path::new("../../../dir1/dir2"))
        );
    }

    #[test]
    fn expand_dots_simple() {
        assert_eq!(Path::new("/foo"), expand_dots(Path::new("/foo/bar/..")));
    }

    #[test]
    fn expand_dots_complex() {
        assert_eq!(
            Path::new("/test"),
            expand_dots(Path::new("/foo/./bar/../../test/././test2/../"))
        );
    }

    #[cfg(windows)]
    mod windows {
        use super::*;

        #[test]
        fn string_with_three_ndots() {
            check_ndots_expansion(r"..\..", "...");
        }

        #[test]
        fn string_with_mixed_ndots_and_chars() {
            check_ndots_expansion(
                r"a...b/./c..d/../e.f/..\..\..//.",
                "a...b/./c..d/../e.f/....//.",
            );
        }

        #[test]
        fn string_with_three_ndots_and_final_slash() {
            check_ndots_expansion(r"..\../", ".../");
        }

        #[test]
        fn string_with_three_ndots_and_garbage() {
            check_ndots_expansion(r"not_a_cmd.../ garbage.*[", "not_a_cmd.../ garbage.*[");
        }
    }

    #[cfg(not(windows))]
    mod non_windows {
        use super::*;
        #[test]
        fn string_with_three_ndots() {
            check_ndots_expansion(r"../..", "...");
        }

        #[test]
        fn string_with_mixed_ndots_and_chars() {
            check_ndots_expansion(
                "a...b/./c..d/../e.f/../../..//.",
                "a...b/./c..d/../e.f/....//.",
            );
        }

        #[test]
        fn string_with_three_ndots_and_final_slash() {
            check_ndots_expansion("../../", ".../");
        }

        #[test]
        fn string_with_three_ndots_and_garbage() {
            // filenames can contain spaces, in these cases the ... .... etc.
            // that are part of a filepath should not be expanded
            check_ndots_expansion("not_a_cmd.../ garbage.*[", "not_a_cmd.../ garbage.*[");
            check_ndots_expansion("/not_a_cmd.../ garbage.*[", "/not_a_cmd.../ garbage.*[");
            check_ndots_expansion("./not_a_cmd.../ garbage.*[", "./not_a_cmd.../ garbage.*[");
            check_ndots_expansion(
                "../../not a cmd.../ garbage.*[",
                ".../not a cmd.../ garbage.*[",
            );
            check_ndots_expansion(
                "../../not a cmd.../ garbage.*[...",
                ".../not a cmd.../ garbage.*[...",
            );
            check_ndots_expansion("../../ not a cmd garbage.*[", ".../ not a cmd garbage.*[");
        }
    }
}
