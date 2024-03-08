use std::path::{is_separator, Component, Path, PathBuf};

use super::helpers;

const EXPAND_STR: &str = if cfg!(windows) { r"..\" } else { "../" };

fn handle_dots_push(string: &mut String, count: u8) {
    if count < 1 {
        return;
    }

    if count == 1 {
        string.push('.');
        return;
    }

    for _ in 0..(count - 1) {
        string.push_str(EXPAND_STR);
    }

    string.pop(); // remove last '/'
}

/// Expands any occurrence of more than two dots into a sequence of ../ (or ..\ on windows), e.g.,
/// "..." into "../..", "...." into "../../../", etc.
pub fn expand_ndots(path: impl AsRef<Path>) -> PathBuf {
    // Check if path is valid UTF-8 and if not, return it as it is to avoid breaking it via string
    // conversion.
    let path_str = match path.as_ref().to_str() {
        Some(s) => s,
        None => return path.as_ref().into(),
    };

    // find if we need to expand any >2 dot paths and early exit if not
    let mut dots_count = 0u8;
    let mut not_separator_before_dot = false;
    let ndots_present = {
        for chr in path_str.chars() {
            if chr == '.' {
                dots_count += 1;
            } else {
                if is_separator(chr) && (dots_count > 2) {
                    // this path component had >2 dots
                    break;
                }
                not_separator_before_dot = !(is_separator(chr) || chr.is_whitespace());
                dots_count = 0;
            }
        }

        dots_count > 2
    };

    if !ndots_present || not_separator_before_dot {
        return path.as_ref().into();
    }

    enum Segment {
        Empty,
        OnlyDots,
        OtherChars,
    }
    let mut dots_count = 0u8;
    let mut path_segment = Segment::Empty;
    let mut expanded = String::with_capacity(path_str.len() + 10);
    for chr in path_str.chars() {
        if chr == '.' {
            if matches!(path_segment, Segment::Empty) {
                path_segment = Segment::OnlyDots;
            }
            dots_count += 1;
        } else {
            if is_separator(chr) {
                if matches!(path_segment, Segment::OnlyDots) {
                    // check for dots expansion only at path component boundaries
                    handle_dots_push(&mut expanded, dots_count);
                    dots_count = 0;
                } else {
                    // if at a path component boundary a secment consists of not only dots
                    // don't expand the dots and only append the appropriate number of .
                    while dots_count > 0 {
                        expanded.push('.');
                        dots_count -= 1;
                    }
                }
                path_segment = Segment::Empty;
            } else {
                // got non-dot within path component => do not expand any dots
                path_segment = Segment::OtherChars;
                while dots_count > 0 {
                    expanded.push('.');
                    dots_count -= 1;
                }
            }
            expanded.push(chr);
        }
    }

    // Here only the final dots without any following characters are handled
    if matches!(path_segment, Segment::OnlyDots) {
        handle_dots_push(&mut expanded, dots_count);
    } else {
        for _ in 0..dots_count {
            expanded.push('.');
        }
    }

    expanded.into()
}

/// Expand "." and ".." into nothing and parent directory, respectively.
pub fn expand_dots(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();

    // Early-exit if path does not contain '.' or '..'
    if !path
        .components()
        .any(|c| std::matches!(c, Component::CurDir | Component::ParentDir))
    {
        return path.into();
    }

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
