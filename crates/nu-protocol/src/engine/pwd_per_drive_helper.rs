use crate::engine::{EngineState, Stack};
#[cfg(windows)]
use {crate::{Span, Value},
     nu_path::get_full_path_name_w,
};
use std::path::{Path, PathBuf};

#[cfg(windows)]
pub mod os_windows {
    use super::*;

    // For maintainer to notify current pwd
    pub mod maintainer {
        use super::*;

        /// when user change current directory, maintainer nofity
        /// PWD-per-drive by calling set_pwd() with current stack and path;
        pub fn set_pwd(stack: &mut Stack, path: &Path) {
            use implementation::{env_var_for_drive, extract_drive_letter};

            if let Some(drive) = extract_drive_letter(path) {
                let value = Value::string(path.to_string_lossy(), Span::unknown());
                stack.add_env_var(env_var_for_drive(drive), value.clone());
            }
        }
    }

    // For file system command usage
    pub mod fs_client {
        use super::*;

        /// file system command implementation can use expand_pwd to expand relate path for a drive
        /// and strip redundant double or single quote like bash
        /// expand_pwd(stack, engine_state, Path::new("''C:''nushell''") ->
        /// Some(PathBuf("C:\\User\\nushell");
        pub fn expand_pwd(
            stack: &Stack,
            engine_state: &EngineState,
            path: &Path,
        ) -> Option<PathBuf> {
            use implementation::{bash_strip_redundant_quotes, extract_drive_letter, get_pwd_on_drive, need_expand};

            if let Some(path_str) = path.to_str() {
                if let Some(path_string) = bash_strip_redundant_quotes(path_str) {
                    if need_expand(&path_string) {
                        if let Some(drive_letter) = extract_drive_letter(Path::new(&path_string)) {
                            let mut base =
                                PathBuf::from(get_pwd_on_drive(stack, engine_state, drive_letter));
                            // Combine PWD with the relative path
                            // need_expand() and extract_drive_letter() all ensure path_str.len() >= 2
                            base.push(&path_string[2..]); // Join PWD with path parts after "C:"
                            return Some(base);
                        }
                    }
                    if path_string != path_str {
                        return Some(PathBuf::from(&path_string));
                    }
                }
            }
            None
        }
    }

    // Implementation for maintainer and fs_client
    pub(in crate::engine::pwd_per_drive_helper) mod implementation {
        use super::*;

        // get pwd for drive:
        // 1. From env_var, if no,
        // 2. From sys_absolute, if no,
        // 3. Construct root path to drives
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

        /// Helper to check if input path is relative path
        /// with drive letter, it can be expanded with PWD-per-drive.
        pub fn need_expand(path: &str) -> bool {
            let chars: Vec<char> = path.chars().collect();
            if chars.len() >= 2 {
                chars[1] == ':' && (chars.len() == 2 || (chars[2] != '/' && chars[2] != '\\'))
            } else {
                false
            }
        }

        /// Get windows env var for drive
        pub fn env_var_for_drive(drive_letter: char) -> String {
            let drive_letter = drive_letter.to_ascii_uppercase();
            format!("={}:", drive_letter)
        }

        /// Helper to extract the drive letter from a path, keep case
        pub fn extract_drive_letter(path: &Path) -> Option<char> {
            path.to_str()
                .and_then(|s| s.chars().next())
                .filter(|c| c.is_ascii_alphabetic())
        }
        /// Ensure a path has a trailing `\\` or '/'
        /// ```
        /// use nu_path::ensure_trailing_delimiter;
        ///
        /// assert_eq!(ensure_trailing_delimiter("E:"), r"E:\");
        /// assert_eq!(ensure_trailing_delimiter(r"e:\"), r"e:\");
        /// assert_eq!(ensure_trailing_delimiter("c:/"), "c:/");
        /// ```
        pub fn ensure_trailing_delimiter(path: &str) -> String {
            if !path.ends_with('\\') && !path.ends_with('/') {
                format!(r"{}\", path)
            } else {
                path.to_string()
            }
        }

        /// Remove redundant quotes as preprocessor for path
        /// #"D:\\"''M''u's 'ic# -> #D:\\Mu's 'ic#
        pub fn bash_strip_redundant_quotes(input: &str) -> Option<String> {
            let mut result = String::new();
            let mut i = 0;
            let chars: Vec<char> = input.chars().collect();

            let mut no_quote_start_pos = 0;
            while i < chars.len() {
                let current_char = chars[i];

                if current_char == '"' || current_char == '\'' {
                    if i > no_quote_start_pos {
                        result.push_str(&input[no_quote_start_pos..i]);
                    }

                    let mut j = i + 1;
                    let mut has_space = false;

                    // Look for the matching quote
                    while j < chars.len() && chars[j] != current_char {
                        if chars[j].is_whitespace() {
                            has_space = true;
                        }
                        j += 1;
                    }

                    // Check if the matching quote exists
                    if j < chars.len() && chars[j] == current_char {
                        if has_space {
                            // Push the entire segment including quotes
                            result.push_str(&input[i..=j]);
                        } else {
                            // Push the inner content without quotes
                            result.push_str(&input[i + 1..j]);
                        }
                        i = j + 1; // Move past the closing quote
                        no_quote_start_pos = i;
                        continue;
                    } else {
                        // No matching quote found, return None
                        return None;
                    }
                }
                i += 1;
            }

            if i > no_quote_start_pos + 1 {
                result.push_str(&input[no_quote_start_pos..i]);
            }
            // Return the result if matching quotes are found
            Some(result)
        }

        /// cmd_strip_all_double_quotes
        /// assert_eq!("t t", cmd_strip_all_double_quotes("t\" \"t\"\""));
        pub fn cmd_strip_all_double_quotes(input: &str) -> String {
            input.replace("\"", "")
        }
    }
}

// For file system command usage
pub mod fs_client {
    use super::*;

    // Helper stub/proxy for nu_path::expand_path_with::<P, Q>(path, relative_to, expand_tilde)
    // Facilitates file system commands to easily gain the ability to expand PWD-per-drive
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

                assert!(need_expand(r"c:nushell\src"));
                assert!(need_expand("C:src/"));
                assert!(need_expand("a:"));
                assert!(need_expand("z:"));
                // Absolute path does not need expand
                assert!(!need_expand(r"c:\"));
                // Unix path does not need expand
                assert!(!need_expand("/usr/bin"));
            }

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
            fn test_os_windows_implementation_extract_drive_letter() {
                use os_windows::implementation::extract_drive_letter;

                assert_eq!(extract_drive_letter(Path::new("C:test")), Some('C'));
                assert_eq!(extract_drive_letter(Path::new(r"d:\temp")), Some('d'));
            }

            #[test]
            fn test_os_windows_implementation_bash_strip_redundant_quotes() {
                use os_windows::implementation::bash_strip_redundant_quotes;

                let input = r#""D:\Music""#;
                let result = Some(r"D:\Music".to_string());
                assert_eq!(result, bash_strip_redundant_quotes(input));

                let input = r#"""""D:\Music"""""#;
                assert_eq!(result, bash_strip_redundant_quotes(input));

                let input = r#""D:\Mus"ic"#;
                assert_eq!(result, bash_strip_redundant_quotes(input));
                let input = r#""D:"\Music"#;
                assert_eq!(result, bash_strip_redundant_quotes(input));

                let input = r#""D":\Music"#;
                assert_eq!(result, bash_strip_redundant_quotes(input));

                let input = r#"""D:\Music"#;
                assert_eq!(result, bash_strip_redundant_quotes(input));

                let input = r#"""''"""D:\Mu sic"""''"""#;
                let result = Some(r#""D:\Mu sic""#.to_string());
                assert_eq!(result, bash_strip_redundant_quotes(input));

                assert_eq!(bash_strip_redundant_quotes(""), Some("".to_string()));
                assert_eq!(bash_strip_redundant_quotes("''"), Some("".to_string()));
                assert_eq!(bash_strip_redundant_quotes("'''"), None);
                assert_eq!(bash_strip_redundant_quotes("'''M'"), Some("M".to_string()));
                assert_eq!(
                    bash_strip_redundant_quotes("'''M '"),
                    Some("'M '".to_string())
                );
                assert_eq!(
                    bash_strip_redundant_quotes(r#"""''"""D:\Mu sic"""''"""#),
                    Some(r#""D:\Mu sic""#.to_string())
                );
            }

            #[test]
            fn test_os_windows_implementation_cmd_strip_all_double_quotes() {
                use os_windows::implementation::cmd_strip_all_double_quotes;

                assert_eq!("t t", cmd_strip_all_double_quotes("t\" \"t\"\""));
            }
        }
    }
}
