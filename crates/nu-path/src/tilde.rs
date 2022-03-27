use std::path::{Path, PathBuf};

fn expand_tilde_with_home(path: impl AsRef<Path>, home: Option<PathBuf>) -> PathBuf {
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
                    // Corner case: `p` is empty;
                    // Don't append extra '/', just keep `h` as is.
                    // This happens because PathBuf.push will always
                    // add a separator if the pushed path is relative,
                    // even if it's empty
                    if p != Path::new("") {
                        h.push(p)
                    }
                }
                h
            }
        }
    }
}

/// Expand tilde ("~") into a home directory if it is the first path component
pub fn expand_tilde(path: impl AsRef<Path>) -> PathBuf {
    // TODO: Extend this to work with "~user" style of home paths
    expand_tilde_with_home(path, dirs_next::home_dir())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::MAIN_SEPARATOR;

    fn check_expanded(s: &str) {
        let home = Path::new("/home");
        let buf = Some(PathBuf::from(home));
        assert!(expand_tilde_with_home(Path::new(s), buf).starts_with(&home));

        // Tests the special case in expand_tilde for "/" as home
        let home = Path::new("/");
        let buf = Some(PathBuf::from(home));
        assert!(!expand_tilde_with_home(Path::new(s), buf).starts_with("//"));
    }

    fn check_not_expanded(s: &str) {
        let home = PathBuf::from("/home");
        let expanded = expand_tilde_with_home(Path::new(s), Some(home));
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

    #[test]
    fn path_does_not_include_trailing_separator() {
        let home = Path::new("/home");
        let buf = Some(PathBuf::from(home));
        let expanded = expand_tilde_with_home(Path::new("~"), buf);
        let expanded_str = expanded.to_str().unwrap();
        assert!(!expanded_str.ends_with(MAIN_SEPARATOR));
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
