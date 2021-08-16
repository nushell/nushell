use std::borrow::Cow;
use std::path::{Path, PathBuf};

use super::util::{cow_map_by_ref, cow_map_str_path};

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

#[cfg(test)]
mod tests {
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
        assert!(expanded == Path::new(s));
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
