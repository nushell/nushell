#[cfg(all(unix, not(target_os = "macos")))]
use pwd::Passwd;
use std::path::{Path, PathBuf};

fn expand_tilde_with_home(path: impl AsRef<Path>, home: Option<PathBuf>) -> PathBuf {
    let path = path.as_ref();

    if !path.starts_with("~") {
        let string = path.to_string_lossy();
        let mut path_as_string = string.as_ref().chars();
        return match path_as_string.next() {
            Some('~') => expand_tilde_with_another_user_home(path),
            _ => path.into(),
        };
    }

    let path_last_char = path.as_os_str().to_string_lossy().chars().last();
    let need_trailing_slash = path_last_char == Some('/') || path_last_char == Some('\\');

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

                    if need_trailing_slash {
                        h.push("");
                    }
                }
                h
            }
        }
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn user_home_dir(username: &str) -> PathBuf {
    let passwd = Passwd::from_name(username);
    match &passwd.ok() {
        Some(Some(dir)) => PathBuf::from(&dir.dir),
        _ => {
            let mut file = String::from("/home/");
            file.push_str(username);
            PathBuf::from(file)
        }
    }
    // PathBuf::from(concat!("/home/", username)),
    // Returns home dir of user.
}

#[cfg(target_os = "macos")]
fn user_home_dir(username: &str) -> PathBuf {
    match dirs_next::home_dir() {
        None => {
            let mut expected_path = String::from("/Users/");
            expected_path.push_str(username);
            let path = Path::new(&expected_path);
            let mut home = PathBuf::new();
            home.push(path);
            home
        }
        Some(user) => {
            let mut expected_path = user;
            expected_path.pop();
            expected_path.push(Path::new(username));
            if expected_path.is_dir() {
                expected_path
            } else {
                let mut expected_path_as_string = String::from("/Users/");
                expected_path_as_string.push_str(username);
                let path = Path::new(&expected_path_as_string);
                let mut home = PathBuf::new();
                home.push(path);
                home
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn user_home_dir(username: &str) -> PathBuf {
    match dirs_next::home_dir() {
        None => {
            let mut expected_path = String::from("C:\\Users\\");
            expected_path.push_str(username);
            let path = Path::new(&expected_path);
            let mut home = PathBuf::new();
            home.push(path);
            home
        }
        Some(user) => {
            let mut expected_path = user;
            expected_path.pop();
            expected_path.push(Path::new(username));
            if expected_path.is_dir() {
                expected_path
            } else {
                let mut expected_path_as_string = String::from("C:\\Users\\");
                expected_path_as_string.push_str(username);
                let path = Path::new(&expected_path_as_string);
                let mut home = PathBuf::new();
                home.push(path);
                home
            }
        }
    }
}

fn expand_tilde_with_another_user_home(path: &Path) -> PathBuf {
    return match path.to_str() {
        Some(file_path) => {
            let mut file = file_path.to_string();
            match file_path.chars().position(|c| c == '/' || c == '\\') {
                None => {
                    file.remove(0);
                    user_home_dir(&file)
                }
                Some(i) => {
                    let (pre_name, rest_of_path) = file.split_at(i);
                    let mut name = pre_name.to_string();
                    let mut rest_path = rest_of_path.to_string();
                    rest_path.remove(0);
                    name.remove(0);
                    let mut path = user_home_dir(&name);
                    path.push(Path::new(&rest_path));
                    path
                }
            }
        }
        None => path.to_path_buf(),
    };
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
        assert!(expand_tilde_with_home(Path::new(s), buf).starts_with(home));

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
