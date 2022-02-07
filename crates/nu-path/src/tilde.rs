use std::path::{Path, PathBuf};

<<<<<<< HEAD
fn expand_tilde_with(path: impl AsRef<Path>, home: Option<PathBuf>) -> PathBuf {
=======
fn expand_tilde_with_home(path: impl AsRef<Path>, home: Option<PathBuf>) -> PathBuf {
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
    let path = path.as_ref();

    if !path.starts_with("~") {
        return path.into();
    }

    match home {
        None => path.into(),
        Some(mut h) => {
            if h == Path::new("/") {
                // Corner case: `h` is a root directory;
                // don't prepend extra `/`, just drop the tilde.
                path.strip_prefix("~").unwrap_or(path).into()
            } else {
                if let Ok(p) = path.strip_prefix("~/") {
                    h.push(p)
                }
                h
            }
        }
    }
}

/// Expand tilde ("~") into a home directory if it is the first path component
pub fn expand_tilde(path: impl AsRef<Path>) -> PathBuf {
    // TODO: Extend this to work with "~user" style of home paths
<<<<<<< HEAD
    expand_tilde_with(path, dirs_next::home_dir())
=======
    expand_tilde_with_home(path, dirs_next::home_dir())
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_expanded(s: &str) {
        let home = Path::new("/home");
        let buf = Some(PathBuf::from(home));
<<<<<<< HEAD
        assert!(expand_tilde_with(Path::new(s), buf).starts_with(&home));
=======
        assert!(expand_tilde_with_home(Path::new(s), buf).starts_with(&home));
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce

        // Tests the special case in expand_tilde for "/" as home
        let home = Path::new("/");
        let buf = Some(PathBuf::from(home));
<<<<<<< HEAD
        assert!(!expand_tilde_with(Path::new(s), buf).starts_with("//"));
=======
        assert!(!expand_tilde_with_home(Path::new(s), buf).starts_with("//"));
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
    }

    fn check_not_expanded(s: &str) {
        let home = PathBuf::from("/home");
<<<<<<< HEAD
        let expanded = expand_tilde_with(Path::new(s), Some(home));
=======
        let expanded = expand_tilde_with_home(Path::new(s), Some(home));
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
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
