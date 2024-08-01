#[cfg(all(unix, not(target_os = "macos"), not(target_os = "android")))]
use pwd::Passwd;
use std::path::{Path, PathBuf};

#[cfg(target_os = "macos")]
const FALLBACK_USER_HOME_BASE_DIR: &str = "/Users";

#[cfg(target_os = "windows")]
const FALLBACK_USER_HOME_BASE_DIR: &str = "C:\\Users\\";

#[cfg(all(unix, not(target_os = "macos"), not(target_os = "android")))]
const FALLBACK_USER_HOME_BASE_DIR: &str = "/home";

#[cfg(all(unix, target_os = "android"))]
const FALLBACK_USER_HOME_BASE_DIR: &str = "/data";

#[cfg(target_os = "android")]
const TERMUX_HOME: &str = "/data/data/com.termux/files/home";

fn expand_tilde_with_home(path: impl AsRef<Path>, home: Option<PathBuf>) -> PathBuf {
    let path = path.as_ref();

    if !path.starts_with("~") {
        let string = path.to_string_lossy();
        let mut path_as_string = string.as_ref().bytes();
        return match path_as_string.next() {
            Some(b'~') => expand_tilde_with_another_user_home(path),
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

fn fallback_home_dir(username: &str) -> PathBuf {
    PathBuf::from_iter([FALLBACK_USER_HOME_BASE_DIR, username])
}

#[cfg(all(unix, not(target_os = "macos"), not(target_os = "android")))]
fn user_home_dir(username: &str) -> PathBuf {
    let passwd = Passwd::from_name(username);
    match &passwd.ok() {
        Some(Some(dir)) => PathBuf::from(&dir.dir),
        _ => fallback_home_dir(username),
    }
}

#[cfg(any(target_os = "android", target_os = "windows", target_os = "macos"))]
fn user_home_dir(username: &str) -> PathBuf {
    use std::path::Component;

    match dirs::home_dir() {
        None => {
            // Termux always has the same home directory
            #[cfg(target_os = "android")]
            if is_termux() {
                return PathBuf::from(TERMUX_HOME);
            }

            fallback_home_dir(username)
        }
        Some(user) => {
            let mut expected_path = user;

            if !cfg!(target_os = "android")
                && expected_path
                    .components()
                    .last()
                    .map(|last| last != Component::Normal(username.as_ref()))
                    .unwrap_or(false)
            {
                expected_path.pop();
                expected_path.push(Path::new(username));
            }

            if expected_path.is_dir() {
                expected_path
            } else {
                fallback_home_dir(username)
            }
        }
    }
}

/// Returns true if the shell is running inside the Termux terminal emulator
/// app.
#[cfg(target_os = "android")]
fn is_termux() -> bool {
    std::env::var("TERMUX_VERSION").is_ok()
}

fn expand_tilde_with_another_user_home(path: &Path) -> PathBuf {
    return match path.to_str() {
        Some(file_path) => {
            let mut file = file_path.to_string();
            match file_path.find(['/', '\\']) {
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
    expand_tilde_with_home(path, dirs::home_dir())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_path_eq;
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
        assert_eq!(expanded, Path::new(s));
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
    fn string_with_tilde_other_user() {
        let s = "~someone/test/";
        let expected = format!("{FALLBACK_USER_HOME_BASE_DIR}/someone/test/");

        assert_eq!(expand_tilde(Path::new(s)), PathBuf::from(expected));
    }

    #[test]
    fn string_with_multi_byte_chars() {
        let s = "~あ/";
        let expected = format!("{FALLBACK_USER_HOME_BASE_DIR}/あ/");

        assert_eq!(expand_tilde(Path::new(s)), PathBuf::from(expected));
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

    // [TODO] Figure out how to reliably test with real users.
    #[test]
    fn user_home_dir_fallback() {
        let user = "nonexistent";
        let expected_home = PathBuf::from_iter([FALLBACK_USER_HOME_BASE_DIR, user]);

        #[cfg(target_os = "android")]
        let expected_home = if is_termux() {
            PathBuf::from(TERMUX_HOME)
        } else {
            expected_home
        };

        let actual_home = super::user_home_dir(user);

        assert_eq!(expected_home, actual_home, "wrong home");
    }

    #[test]
    #[cfg(not(windows))]
    fn expand_tilde_preserve_trailing_slash() {
        let path = PathBuf::from("~/foo/");
        let home = PathBuf::from("/home");

        let actual = expand_tilde_with_home(path, Some(home));
        assert_path_eq!(actual, "/home/foo/");
    }
    #[test]
    #[cfg(windows)]
    fn expand_tilde_preserve_trailing_slash() {
        let path = PathBuf::from("~\\foo\\");
        let home = PathBuf::from("C:\\home");

        let actual = expand_tilde_with_home(path, Some(home));
        assert_path_eq!(actual, "C:\\home\\foo\\");
    }
}
