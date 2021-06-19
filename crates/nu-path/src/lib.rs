use std::borrow::Cow;
use std::io;
use std::path::{Component, Path, PathBuf};

// Utility for applying a function that can only be called on the borrowed type of the Cow
// and also returns a ref. If the Cow is a borrow, we can return the same borrow but an
// owned value needs extra handling because the returned valued has to be owned as well
pub fn cow_map_by_ref<B, O, F>(c: Cow<'_, B>, f: F) -> Cow<'_, B>
where
    B: ToOwned<Owned = O> + ?Sized,
    O: AsRef<B>,
    F: FnOnce(&B) -> &B,
{
    match c {
        Cow::Borrowed(b) => Cow::Borrowed(f(b)),
        Cow::Owned(o) => Cow::Owned(f(o.as_ref()).to_owned()),
    }
}

// Utility for applying a function over Cow<'a, Path> over a Cow<'a, str> while avoiding unnecessary conversions
fn cow_map_str_path<'a, F>(c: Cow<'a, str>, f: F) -> Cow<'a, str>
where
    F: FnOnce(Cow<'a, Path>) -> Cow<'a, Path>,
{
    let ret = match c {
        Cow::Borrowed(b) => f(Cow::Borrowed(Path::new(b))),
        Cow::Owned(o) => f(Cow::Owned(PathBuf::from(o))),
    };

    match ret {
        Cow::Borrowed(expanded) => expanded.to_string_lossy(),
        Cow::Owned(expanded) => Cow::Owned(expanded.to_string_lossy().to_string()),
    }
}

// Utility for applying a function over Cow<'a, str> over a Cow<'a, Path> while avoiding unnecessary conversions
fn cow_map_path_str<'a, F>(c: Cow<'a, Path>, f: F) -> Cow<'a, Path>
where
    F: FnOnce(Cow<'a, str>) -> Cow<'a, str>,
{
    let ret = match c {
        Cow::Borrowed(path) => f(path.to_string_lossy()),
        Cow::Owned(buf) => f(Cow::Owned(buf.to_string_lossy().to_string())),
    };

    match ret {
        Cow::Borrowed(expanded) => Cow::Borrowed(Path::new(expanded)),
        Cow::Owned(expanded) => Cow::Owned(PathBuf::from(expanded)),
    }
}

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

// Expands any occurence of more than two dots into a sequence of ../ (or ..\ on windows), e.g.
// ... into ../..
// .... into ../../../
fn expand_ndots_string(path: Cow<'_, str>) -> Cow<'_, str> {
    use std::path::is_separator;
    // find if we need to expand any >2 dot paths and early exit if not
    let mut dots_count = 0u8;
    let ndots_present = {
        for chr in path.chars() {
            if chr == '.' {
                dots_count += 1;
            } else {
                if is_separator(chr) && (dots_count > 2) {
                    // this path component had >2 dots
                    break;
                }

                dots_count = 0;
            }
        }

        dots_count > 2
    };

    if !ndots_present {
        return path;
    }

    let mut dots_count = 0u8;
    let mut expanded = String::new();
    for chr in path.chars() {
        if chr == '.' {
            dots_count += 1;
        } else {
            if is_separator(chr) {
                // check for dots expansion only at path component boundaries
                handle_dots_push(&mut expanded, dots_count);
                dots_count = 0;
            } else {
                // got non-dot within path component => do not expand any dots
                while dots_count > 0 {
                    expanded.push('.');
                    dots_count -= 1;
                }
            }
            expanded.push(chr);
        }
    }

    handle_dots_push(&mut expanded, dots_count);

    expanded.into()
}

// Expands any occurence of more than two dots into a sequence of ../ (or ..\ on windows), e.g.
// ... into ../..
// .... into ../../../
fn expand_ndots(path: Cow<'_, Path>) -> Cow<'_, Path> {
    cow_map_path_str(path, expand_ndots_string)
}

pub fn absolutize<P, Q>(relative_to: P, path: Q) -> PathBuf
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let path = if path.as_ref() == Path::new(".") {
        // Joining a Path with '.' appends a '.' at the end, making the prompt
        // more ugly - so we don't do anything, which should result in an equal
        // path on all supported systems.
        relative_to.as_ref().to_owned()
    } else if path.as_ref().starts_with("~") {
        expand_tilde(Cow::Borrowed(path.as_ref())).to_path_buf()
    } else {
        relative_to.as_ref().join(path)
    };

    let (relative_to, path) = {
        let components: Vec<_> = path.components().collect();
        let separator = components
            .iter()
            .enumerate()
            .find(|(_, c)| c == &&Component::CurDir || c == &&Component::ParentDir);

        if let Some((index, _)) = separator {
            let (absolute, relative) = components.split_at(index);
            let absolute: PathBuf = absolute.iter().collect();
            let relative: PathBuf = relative.iter().collect();

            (absolute, relative)
        } else {
            (
                relative_to.as_ref().to_path_buf(),
                components.iter().collect::<PathBuf>(),
            )
        }
    };

    let path = if path.is_relative() {
        let mut result = relative_to;
        path.components().for_each(|component| match component {
            Component::ParentDir => {
                result.pop();
            }
            Component::Normal(normal) => result.push(normal),
            _ => {}
        });

        result
    } else {
        path
    };

    dunce::simplified(&path).to_path_buf()
}

pub fn canonicalize<P, Q>(relative_to: P, path: Q) -> io::Result<PathBuf>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let absolutized = absolutize(&relative_to, path);
    let path = match std::fs::read_link(&absolutized) {
        Ok(resolved) => {
            let parent = absolutized.parent().unwrap_or(&absolutized);
            absolutize(parent, resolved)
        }

        Err(e) => {
            if absolutized.exists() {
                absolutized
            } else {
                return Err(e);
            }
        }
    };

    Ok(dunce::simplified(&path).to_path_buf())
}

// Expansion logic lives here to enable testing without depending on dirs-next
fn expand_tilde_with(path: Cow<'_, Path>, home: Option<PathBuf>) -> Cow<'_, Path> {
    if !path.starts_with("~") {
        return path;
    }

    match home {
        None => path,
        Some(mut h) => {
            if h == Path::new("/") {
                // Corner case: `h` root directory;
                // don't prepend extra `/`, just drop the tilde.
                cow_map_by_ref(path, |p: &Path| {
                    p.strip_prefix("~").expect("cannot strip ~ prefix")
                })
            } else {
                h.push(path.strip_prefix("~/").expect("cannot strip ~/ prefix"));
                Cow::Owned(h)
            }
        }
    }
}

pub fn expand_tilde(path: Cow<'_, Path>) -> Cow<'_, Path> {
    expand_tilde_with(path, dirs_next::home_dir())
}

pub fn expand_tilde_string(path: Cow<'_, str>) -> Cow<'_, str> {
    cow_map_str_path(path, expand_tilde)
}

// Remove "." and ".." in a path. Prefix ".." are not removed as we don't have access to the
// current dir. This is merely 'string manipulation'. Does not handle "...+", see expand_ndots for that
pub fn resolve_dots(path: Cow<'_, Path>) -> Cow<'_, Path> {
    debug_assert!(!path.components().any(|c| std::matches!(c, Component::Normal(os_str) if os_str.to_string_lossy().starts_with("..."))), "Unexpected ndots!");
    if !path
        .components()
        .any(|c| std::matches!(c, Component::CurDir | Component::ParentDir))
    {
        return path;
    }

    let mut result = PathBuf::with_capacity(path.as_os_str().len());

    // Only pop/skip path elements if the previous one was an actual path element
    let prev_is_normal = |p: &Path| -> bool {
        p.components()
            .next_back()
            .map(|c| std::matches!(c, Component::Normal(_)))
            .unwrap_or(false)
    };
    path.as_ref()
        .components()
        .for_each(|component| match component {
            Component::ParentDir if prev_is_normal(&result) => {
                result.pop();
            }
            Component::CurDir if prev_is_normal(&result) => {}
            _ => result.push(component),
        });

    Cow::Owned(dunce::simplified(&result).to_path_buf())
}

// Expands ~ to home and shortens paths by removing unecessary ".." and "."
// where possible. Also expands "...+" appropriately.
pub fn expand_path(path: Cow<'_, Path>) -> Cow<'_, Path> {
    let path = expand_tilde(path);
    let path = expand_ndots(path);
    resolve_dots(path)
}

pub fn expand_path_string(path: Cow<'_, str>) -> Cow<'_, str> {
    cow_map_str_path(path, expand_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn absolutize_two_dots() {
        let relative_to = Path::new("/foo/bar");
        let path = Path::new("..");

        assert_eq!(
            PathBuf::from("/foo"), // missing path
            absolutize(relative_to, path)
        );
    }

    #[test]
    fn absolutize_with_curdir() {
        let relative_to = Path::new("/foo");
        let path = Path::new("./bar/./baz");

        assert!(!absolutize(relative_to, path)
            .to_str()
            .unwrap()
            .contains('.'));
    }

    #[test]
    fn canonicalize_should_succeed() -> io::Result<()> {
        let relative_to = Path::new("/foo/bar");
        let path = Path::new("../..");

        assert_eq!(
            PathBuf::from("/"), // existing path
            canonicalize(relative_to, path)?,
        );

        Ok(())
    }

    #[test]
    fn canonicalize_should_fail() {
        let relative_to = Path::new("/foo/bar/baz"); // '/foo' is missing
        let path = Path::new("../..");

        assert!(canonicalize(relative_to, path).is_err());
    }

    fn check_ndots_expansion(expected: &str, s: &str) {
        let expanded = expand_ndots(Cow::Borrowed(Path::new(s)));
        // If we don't expect expansion, verify that we get a borrow back and no PathBuf creation has been made
        if expected == s {
            assert!(
                std::matches!(expanded, Cow::Borrowed(_)),
                "No PathBuf should be needed here (unnecessary allocation)"
            );
        }
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
    fn resolve_dots_double_dots_no_change() {
        // Can't resolve this as we don't know our parent dir
        assert_eq!(Path::new(".."), resolve_dots(Path::new("..").into()));
    }

    #[test]
    fn resolve_dots_single_dot_no_change() {
        // Can't resolve this as we don't know our current dir
        assert_eq!(Path::new("."), resolve_dots(Path::new(".").into()));
    }

    #[test]
    fn resolve_dots_multi_single_dots_no_change() {
        assert_eq!(Path::new("././."), resolve_dots(Path::new("././.").into()));
    }

    #[test]
    fn resolve_multi_double_dots_no_change() {
        assert_eq!(
            Path::new("../../../"),
            resolve_dots(Path::new("../../../").into())
        );
    }

    #[test]
    fn resolve_dots_no_change_with_dirs() {
        // Can't resolve this as we don't know our parent dir
        assert_eq!(
            Path::new("../../../dir1/dir2/"),
            resolve_dots(Path::new("../../../dir1/dir2").into())
        );
    }

    #[test]
    fn resolve_dots_simple() {
        assert_eq!(
            Path::new("/foo"),
            resolve_dots(Path::new("/foo/bar/..").into())
        );
    }

    #[test]
    fn resolve_dots_complex() {
        assert_eq!(
            Path::new("/test"),
            resolve_dots(Path::new("/foo/./bar/../../test/././test2/../").into())
        );
    }

    // Windows tests
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
            check_ndots_expansion(r"ls ..\../ garbage.*[", "ls .../ garbage.*[");
        }
    }

    // non-Windows tests
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
            check_ndots_expansion("ls ../../ garbage.*[", "ls .../ garbage.*[");
        }
    }

    mod tilde {
        use super::*;

        fn check_expanded(s: &str) {
            let home = Path::new("/home");
            let buf = Some(PathBuf::from(home));
            assert!(expand_tilde_with(Cow::Borrowed(Path::new(s)), buf).starts_with(&home));

            // Tests the special case in expand_tilde for "/" as home
            let home = Path::new("/");
            let buf = Some(PathBuf::from(home));
            assert!(!expand_tilde_with(Cow::Borrowed(Path::new(s)), buf).starts_with("//"));
        }

        fn check_not_expanded(s: &str) {
            let home = PathBuf::from("/home");
            let expanded = expand_tilde_with(Cow::Borrowed(Path::new(s)), Some(home));
            assert!(
                std::matches!(expanded, Cow::Borrowed(_)),
                "No PathBuf should be needed here (unecessary allocation)"
            );
            assert!(&expanded == Path::new(s));
        }

        #[test]
        fn string_with_tilde() {
            check_expanded("~");
        }

        #[test]
        fn string_with_tilde_forward_slash() {
            check_expanded("~/test/");
        }

        #[test]
        fn string_with_tilde_double_forward_slash() {
            check_expanded("~//test/");
        }

        #[test]
        fn does_not_expand_tilde_if_tilde_is_not_first_character() {
            check_not_expanded("1~1");
        }

        #[cfg(windows)]
        #[test]
        fn string_with_tilde_backslash() {
            check_expanded("~\\test/test2/test3");
        }

        #[cfg(windows)]
        #[test]
        fn string_with_double_tilde_backslash() {
            check_expanded("~\\\\test\\test2/test3");
        }
    }
}
