use crate::engine::{EngineState, Stack};
#[cfg(windows)]
use crate::{Span, Value};
use std::path::{Path, PathBuf};

// For file system command usage
pub mod fs_client {
    use super::*;

    /// Proxy/Wrapper for
    /// nu_path::expand_path_with<P, Q>(path, relative_to, expand_tilde);
    ///
    /// Usually if a command opens one file or directory, it uses
    /// nu_path::expand_path_with::<P, Q>(p, r, t) to expand '~','.' etc.; replacing it with
    /// nu_protocol::engine::fs_client::expand_path_with(stack, engine_state, p, r t) will
    /// first check if the path is relative for a drive;
    /// Commands that accept multiple files/directories as parameters usually depend on Glob,
    /// after near future revised Glob collection implementation done, all file system commands
    /// will support PWD-per-drive.
    pub fn expand_path_with<P, Q>(
        _stack: &Stack,
        _engine_state: &EngineState,
        path: P,
        relative_to: Q,
        expand_tilde: bool,
    ) -> PathBuf
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        #[cfg(windows)]
        if let Some(abs_path) =
            os_windows::fs_client::expand_pwd(_stack, _engine_state, path.as_ref())
        {
            return abs_path;
        }

        nu_path::expand_path_with::<P, Q>(path, relative_to, expand_tilde)
    }
}

#[cfg(windows)]
pub mod os_windows {
    use super::*;

    /// For maintainer to notify current pwd
    pub mod maintainer {
        use super::*;

        /// When user change current directory, maintainer notifies
        /// PWD-per-drive by calling set_pwd() with current stack and path;
        pub fn set_pwd(stack: &mut Stack, path: &Path) {
            use implementation::{env_var_for_drive, extract_drive_letter};

            if let Some(path_str) = path.to_str() {
                if let Some(drive) = extract_drive_letter(path_str) {
                    stack.add_env_var(
                        env_var_for_drive(drive),
                        Value::string(path_str, Span::unknown()).clone(),
                    );
                }
            }
        }
    }

    /// For file system command usage
    pub mod fs_client {
        use super::*;

        /// File system command implementation can also directly use expand_pwd
        /// to expand relate path for a drive and strip redundant double or
        /// single quote like bash.
        /// cd "''C:''nushell''"
        /// C:\Users\nushell>
        pub fn expand_pwd(
            stack: &Stack,
            engine_state: &EngineState,
            path: &Path,
        ) -> Option<PathBuf> {
            use implementation::{get_pwd_on_drive, need_expand};

            if let Some(path_str) = path.to_str() {
                if let Some(drive_letter) = need_expand(path_str) {
                    let mut base =
                        PathBuf::from(get_pwd_on_drive(stack, engine_state, drive_letter));
                    // need_expand() ensures path_str.len() >= 2
                    base.push(&path_str[2..]); // Join PWD with path parts after "C:"
                    return Some(base);
                }
            }
            None
        }
    }

    /// Implementation for maintainer and fs_client
    pub(in crate::engine::pwd_per_drive) mod implementation {
        use super::*;

        /// Windows env var for drive
        /// essential for integration with windows native shell CMD/PowerShell
        /// and the core mechanism for supporting PWD-per-drive with nushell's
        /// powerful layered environment system.
        /// Returns uppercased "=X:".
        pub fn env_var_for_drive(drive_letter: char) -> String {
            let drive_letter = drive_letter.to_ascii_uppercase();
            format!("={}:", drive_letter)
        }

        /// get pwd for drive:
        /// 1. From env_var, if no,
        /// 2. From sys_absolute, if no,
        /// 3. Construct root path to drives
        pub fn get_pwd_on_drive(
            stack: &Stack,
            engine_state: &EngineState,
            drive_letter: char,
        ) -> String {
            let env_var_for_drive = env_var_for_drive(drive_letter);
            let mut abs_pwd: Option<String> = None;
            if let Some(pwd) = stack.get_env_var(engine_state, &env_var_for_drive) {
                if let Ok(pwd_string) = pwd.clone().into_string() {
                    abs_pwd = Some(pwd_string);
                }
            }
            if abs_pwd.is_none() {
                if let Some(sys_pwd) = get_full_path_name_w(&format!("{}:", drive_letter)) {
                    abs_pwd = Some(sys_pwd);
                }
            }
            if let Some(pwd) = abs_pwd {
                ensure_trailing_delimiter(&pwd)
            } else {
                format!(r"{}:\", drive_letter)
            }
        }

        /// Check if input path is relative path for drive letter,
        /// which should be expanded with PWD-per-drive.
        /// Returns Some(drive_letter) or None, drive_letter is upper case.
        pub fn need_expand(path: &str) -> Option<char> {
            let chars: Vec<char> = path.chars().collect();
            if chars.len() == 2 || (chars.len() > 2 && chars[2] != '/' && chars[2] != '\\') {
                extract_drive_letter(path)
            } else {
                None
            }
        }

        /// Extract the drive letter from a path, return uppercased
        /// drive letter or None
        pub fn extract_drive_letter(path: &str) -> Option<char> {
            let chars: Vec<char> = path.chars().collect();
            if chars.len() >= 2 && chars[0].is_ascii_alphabetic() && chars[1] == ':' {
                Some(chars[0].to_ascii_uppercase())
            } else {
                None
            }
        }

        /// Ensure a path has a trailing `\\` or '/'
        pub fn ensure_trailing_delimiter(path: &str) -> String {
            if !path.ends_with('\\') && !path.ends_with('/') {
                format!(r"{}\", path)
            } else {
                path.to_string()
            }
        }

        /// get_full_path_name_w
        /// Call windows system API (via omnipath crate) to expand
        /// absolute path
        pub fn get_full_path_name_w(path_str: &str) -> Option<String> {
            use omnipath::sys_absolute;
            use std::path::Path;

            if let Ok(path_sys_abs) = sys_absolute(Path::new(path_str)) {
                Some(path_sys_abs.to_str()?.to_string())
            } else {
                None
            }
        }
    }
}

#[cfg(windows)]
#[cfg(test)] // test only for windows
mod tests {
    use super::*;

    mod fs_client_test {
        use super::*;

        #[test]
        fn test_fs_client_expand_path_with() {
            let mut stack = Stack::new();
            let path_str = r"c:\users\nushell";
            let path = Path::new(path_str);
            os_windows::maintainer::set_pwd(&mut stack, path);
            let engine_state = EngineState::new();

            let rel_path = Path::new("c:.config");
            let result = format!(r"{path_str}\.config");
            assert_eq!(
                Some(result.as_str()),
                fs_client::expand_path_with(
                    &stack,
                    &engine_state,
                    rel_path,
                    Path::new(path_str),
                    false
                )
                .as_path()
                .to_str()
            );
        }
    }

    mod os_windows_tests {
        use super::*;

        #[test]
        fn test_os_windows_maintainer_set_pwd() {
            let mut stack = Stack::new();
            let path_str = r"c:\uesrs\nushell";
            let path = Path::new(path_str);
            os_windows::maintainer::set_pwd(&mut stack, path);
            let engine_state = EngineState::new();
            assert_eq!(
                stack
                    .get_env_var(
                        &engine_state,
                        &os_windows::implementation::env_var_for_drive('c')
                    )
                    .unwrap()
                    .clone()
                    .into_string()
                    .unwrap(),
                path_str.to_string()
            );
        }

        #[test]
        fn test_os_windows_fs_client_expand_pwd() {
            let mut stack = Stack::new();
            let path_str = r"c:\users\nushell";
            let path = Path::new(path_str);
            os_windows::maintainer::set_pwd(&mut stack, path);
            let engine_state = EngineState::new();

            let rel_path = Path::new("c:.config");
            let result = format!(r"{path_str}\.config");
            assert_eq!(
                Some(result.as_str()),
                os_windows::fs_client::expand_pwd(&stack, &engine_state, rel_path)
                    .unwrap()
                    .as_path()
                    .to_str()
            );
        }

        mod implementation_test {
            use super::*;

            #[test]
            fn test_os_windows_implementation_env_var_for_drive() {
                use os_windows::implementation::env_var_for_drive;

                for drive_letter in 'A'..='Z' {
                    assert_eq!(env_var_for_drive(drive_letter), format!("={drive_letter}:"));
                }
                for drive_letter in 'a'..='z' {
                    assert_eq!(
                        env_var_for_drive(drive_letter),
                        format!("={}:", drive_letter.to_ascii_uppercase())
                    );
                }
            }

            #[test]
            fn test_os_windows_implementation_get_pwd_on_drive() {
                let mut stack = Stack::new();
                let path_str = r"c:\users\nushell";
                let path = Path::new(path_str);
                os_windows::maintainer::set_pwd(&mut stack, path);
                let engine_state = EngineState::new();
                let result = format!(r"{path_str}\");
                assert_eq!(
                    result,
                    os_windows::implementation::get_pwd_on_drive(&stack, &engine_state, 'c')
                );
            }

            #[test]
            fn test_os_windows_implementation_need_expand() {
                use os_windows::implementation::need_expand;

                assert_eq!(need_expand(r"c:nushell\src"), Some('C'));
                assert_eq!(need_expand("C:src/"), Some('C'));
                assert_eq!(need_expand("a:"), Some('A'));
                assert_eq!(need_expand("z:"), Some('Z'));
                // Absolute path does not need expand
                assert_eq!(need_expand(r"c:\"), None);
                // Unix path does not need expand
                assert_eq!(need_expand("/usr/bin"), None);
                // Invalid path on drive
                assert_eq!(need_expand("1:usr/bin"), None);
            }

            #[test]
            fn test_os_windows_implementation_extract_drive_letter() {
                use os_windows::implementation::extract_drive_letter;

                assert_eq!(extract_drive_letter("C:test"), Some('C'));
                assert_eq!(extract_drive_letter(r"d:\temp"), Some('D'));
                assert_eq!(extract_drive_letter(r"1:temp"), None);
            }

            #[test]
            fn test_os_windows_implementation_ensure_trailing_delimiter() {
                use os_windows::implementation::ensure_trailing_delimiter;

                assert_eq!(ensure_trailing_delimiter("E:"), r"E:\");
                assert_eq!(ensure_trailing_delimiter(r"e:\"), r"e:\");
                assert_eq!(ensure_trailing_delimiter("c:/"), "c:/");
            }

            #[test]
            fn test_os_windows_implementation_get_full_path_name_w() {
                use os_windows::implementation::get_full_path_name_w;

                let result = get_full_path_name_w("C:");
                assert!(result.is_some());
                let path = result.unwrap();
                assert!(path.starts_with(r"C:\"));

                let result = get_full_path_name_w(r"c:nushell\src");
                assert!(result.is_some());
                let path = result.unwrap();
                assert!(path.starts_with(r"C:\") || path.starts_with(r"c:\"));
                assert!(path.ends_with(r"nushell\src"));
            }
        }
    }
}
