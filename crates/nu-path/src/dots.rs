use std::path::{Component, Path, PathBuf};

use super::helpers;

/// Normalize the path, expanding occurances of n-dots.
///
/// It performs the same normalization as `Path::components()`, except it also expands n-dots,
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
    for component in path.components() {
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

/// Normalize the path, expanding occurances of "." and "..".
///
/// It performs the same normalization as `Path::components()`, except it also expands ".."
/// when its preceding component is a normal component, ignoring the possibility of symlinks.
/// In other words, it operates on the lexical structure of the path.
///
/// The resulting path will use platform-specific path separators, regardless of what path separators was used in the input.
pub fn expand_dots(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();

    let mut result = PathBuf::with_capacity(path.as_os_str().len());

    // Only pop/skip path elements if the previous one was an actual path element
    let prev_is_normal = |p: &Path| -> bool {
        p.components()
            .next_back()
            .map(|c| std::matches!(c, Component::Normal(_)))
            .unwrap_or(false)
    };

    path.components().for_each(|component| match component {
        Component::ParentDir if prev_is_normal(&result) => {
            result.pop();
        }
        Component::CurDir if prev_is_normal(&result) => {}
        _ => result.push(component),
    });

    helpers::simiplified(&result)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Path equality in Rust is defined by comparing their `components()`.
    /// However, `components()` will perform its own normalization, which is not ideal for testing.
    /// Avoid `assert_eq!` in tests; use `assert_path_eq!` instead, which converts path to string before comparision.
    /// It accepts PathBuf, Path, String, and &str.
    macro_rules! assert_path_eq {
        ($left:expr, $right:expr $(,)?) => {
            assert_eq!(
                AsRef::<Path>::as_ref(&$left).to_str().unwrap(),
                AsRef::<Path>::as_ref(&$right).to_str().unwrap()
            )
        };
    }
    macro_rules! assert_path_ne {
        ($left:expr, $right:expr $(,)?) => {
            assert_ne!(
                AsRef::<Path>::as_ref(&$left).to_str().unwrap(),
                AsRef::<Path>::as_ref(&$right).to_str().unwrap()
            )
        };
    }

    /// Fixes the path separator for test cases, so that you don't need to write the same test case twice.
    ///
    /// For example, "./foo\bar" is converted to ".\foo\bar" on Windows, and "./foo/bar" on other platforms.
    #[cfg(windows)]
    fn platform_path(path: &str) -> String {
        path.replace(r"/", r"\")
    }
    #[cfg(not(windows))]
    fn platform_path(path: &str) -> String {
        path.replace(r"\", r"/")
    }

    #[test]
    fn assert_path_eq_works() {
        assert_path_eq!(PathBuf::from("/foo/bar"), Path::new("/foo/bar"));
        assert_path_eq!(PathBuf::from("/foo/bar"), String::from("/foo/bar"));
        assert_path_eq!(PathBuf::from("/foo/bar"), "/foo/bar");
        assert_path_eq!(Path::new("/foo/bar"), String::from("/foo/bar"));
        assert_path_eq!(Path::new("/foo/bar"), "/foo/bar");
        assert_path_eq!(Path::new(r"\foo\bar"), r"\foo\bar");

        assert_path_ne!(PathBuf::from("/foo/bar/."), Path::new("/foo/bar"));
        assert_path_ne!(PathBuf::from("/foo/bar/."), String::from("/foo/bar"));
        assert_path_ne!(PathBuf::from("/foo/bar/."), "/foo/bar");
        assert_path_ne!(Path::new("/foo/./bar"), String::from("/foo/bar"));
        assert_path_ne!(Path::new("/foo/./bar"), "/foo/bar");
        assert_path_ne!(Path::new(r"\foo\bar"), r"/foo/bar");
        assert_path_ne!(Path::new(r"/foo/bar"), r"\foo\bar");
    }

    #[test]
    fn platform_path_works() {
        #[cfg(windows)]
        {
            assert_eq!(platform_path(r"/foo/bar"), r"\foo\bar");
            assert_eq!(platform_path(r"C:\foo\bar"), r"C:\foo\bar");
        }

        #[cfg(not(windows))]
        {
            assert_eq!(platform_path(r"/foo/bar"), r"/foo/bar");
            assert_eq!(platform_path(r"C:\foo\bar"), r"C:/foo/bar");
        }
    }

    #[test]
    fn expand_two_dots() {
        let path = Path::new("/foo/bar/..");
        assert_path_eq!(platform_path("/foo"), expand_dots(path));
    }

    #[test]
    fn expand_dots_with_curdir() {
        let path = Path::new("/foo/./bar/./baz");
        assert_path_eq!(platform_path("/foo/bar/baz"), expand_dots(path));
    }

    // track_caller refers, in the panic-message, to the line of the function call and not
    // inside of the function, which is nice for a test-helper-function
    #[track_caller]
    fn check_ndots_expansion(expected: &str, s: &str) {
        let expanded = expand_ndots(s);
        assert_path_eq!(platform_path(expected), expanded);
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
        check_ndots_expansion("../../../..", ".../...");
        check_ndots_expansion("../../file name/../..", ".../file name/...");
        check_ndots_expansion("../../../file name/../../..", "..../file name/....");
    }

    #[test]
    fn expand_dots_double_dots_no_change() {
        // Can't resolve this as we don't know our parent dir
        assert_path_eq!("..", expand_dots(".."));
    }

    #[test]
    fn expand_dots_single_dot_no_change() {
        // Can't resolve this as we don't know our current dir
        assert_path_eq!(".", expand_dots("."));
    }

    #[test]
    fn expand_dots_multi_single_dots() {
        assert_path_eq!(".", expand_dots("././."));
    }

    #[test]
    fn expand_multi_double_dots_no_change() {
        assert_path_eq!(platform_path("../../.."), expand_dots("../../../"));
    }

    #[test]
    fn expand_dots_no_change_with_dirs() {
        // Can't resolve this as we don't know our parent dir
        assert_path_eq!(
            platform_path("../../../dir1/dir2"),
            expand_dots("../../../dir1/dir2"),
        );
    }

    #[test]
    fn expand_dots_simple() {
        assert_path_eq!(platform_path("/foo"), expand_dots("/foo/bar/.."));
    }

    #[test]
    fn expand_dots_complex() {
        assert_path_eq!(
            platform_path("/test"),
            expand_dots("/foo/./bar/../../test/././test2/../"),
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
            check_ndots_expansion(r"a...b/c..d/../e.f/../../..", "a...b/./c..d/../e.f/....//.");
        }

        #[test]
        fn string_with_three_ndots_and_final_slash() {
            check_ndots_expansion(r"..\..", ".../");
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
