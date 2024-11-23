#[cfg(windows)]
pub mod _impl {
    use std::path::{Path, PathBuf};
    /// Usage for pwd_per_drive on windows
    ///
    /// Upon change PWD, call set_pwd_per_drive() with absolute path
    ///
    /// Call expand_pwd_per_drive() with relative path to get absolution path
    ///
    /// ```
    /// use std::path::{Path, PathBuf};
    /// use nu_path::{expand_pwd, set_pwd};
    ///
    /// // Set PWD for drive C
    /// set_pwd(Path::new(r"C:\Users\Home")).unwrap();
    ///
    /// // Expand a relative path
    /// let expanded = expand_pwd(Path::new("C:test"));
    /// assert_eq!(expanded, Some(PathBuf::from(r"C:\Users\Home\test")));
    ///
    /// // Will NOT expand an absolute path
    /// let expanded = expand_pwd(Path::new(r"C:\absolute\path"));
    /// assert_eq!(expanded, None);
    ///
    /// // Expand with no drive letter
    /// let expanded = expand_pwd(Path::new(r"\no_drive"));
    /// assert_eq!(expanded, None);
    ///
    /// // Expand with no PWD set for the drive
    /// let expanded = expand_pwd(Path::new("D:test"));
    /// assert_eq!(expanded, Some(PathBuf::from(r"D:\test")));
    /// ```
    pub mod singleton {
        use super::*;

        /// set_pwd_per_drive
        /// Record PWD for drive, path must be absolute path
        /// return Ok(()) if succeeded, otherwise error message
        pub fn set_pwd(_path: &Path) -> Result<(), String> {
            if let Ok(mut pwd_per_drive) = get_drive_pwd_map().lock() {
                pwd_per_drive.set_pwd(_path)
            } else {
                Err("Failed to lock map".to_string())
            }
        }

        /// expand_pwe_per_drive
        /// Input relative path, expand PWD-per-drive to construct absolute path
        /// return PathBuf for absolute path, None if input path is invalid.
        pub fn expand_pwd(_path: &Path) -> Option<PathBuf> {
            if need_expand_pwd_per_drive(_path) {
                if let Ok(mut pwd_per_drive) = get_drive_pwd_map().lock() {
                    return pwd_per_drive.expand_path(_path);
                }
            }
            None
        }

        /// Helper only used on Windows, if input path is relative path
        /// with drive letter, it can be expanded with PWD-per-drive.
        fn need_expand_pwd_per_drive(_path: &Path) -> bool {
            if let Some(path_str) = _path.to_str() {
                let chars: Vec<char> = path_str.chars().collect();
                if chars.len() >= 2 {
                    return chars[1] == ':'
                        && (chars.len() == 2 || (chars[2] != '/' && chars[2] != '\\'));
                }
            }
            false
        }
    }

    #[test]
    fn test_usage_for_pwd_per_drive() {
        use super::_impl::singleton::{expand_pwd, set_pwd};
        // Set PWD for drive F
        assert!(set_pwd(Path::new(r"F:\Users\Home")).is_ok());

        // Expand a relative path
        let expanded = expand_pwd(Path::new("f:test"));
        assert_eq!(expanded, Some(PathBuf::from(r"F:\Users\Home\test")));

        // Will NOT expand an absolute path
        let expanded = expand_pwd(Path::new(r"F:\absolute\path"));
        assert_eq!(expanded, None);

        // Expand with no drive letter
        let expanded = expand_pwd(Path::new(r"\no_drive"));
        assert_eq!(expanded, None);

        // Expand with no PWD set for the drive
        let expanded = expand_pwd(Path::new("G:test"));
        assert_eq!(expanded, Some(PathBuf::from(r"G:\test")));
    }

    struct Drive2PWD {
        map: [Option<String>; 26], // Fixed-size array for A-Z
    }

    impl Drive2PWD {
        pub fn new() -> Self {
            Drive2PWD {
                map: Default::default(), // Initialize all to `None`
            }
        }

        /// Set the PWD for the drive letter in the absolute path.
        /// Return String for error description.
        pub fn set_pwd(&mut self, path: &Path) -> Result<(), String> {
            if let (Some(drive_letter), Some(path_str)) =
                (Self::extract_drive_letter(path), path.to_str())
            {
                self.map[drive_letter as usize - 'A' as usize] = Some(path_str.to_string());
                return Ok(());
            }
            Err(format!("Invalid path: {}", path.display()))
        }

        /// Get the PWD for drive, if not yet, ask GetFullPathNameW(),
        /// or else return default r"X:\".
        fn get_pwd(&mut self, drive: char) -> Option<String> {
            if drive.is_ascii_alphabetic() {
                let drive = drive.to_ascii_uppercase();
                let index = drive as usize - 'A' as usize;
                Some(self.map[index].clone().unwrap_or_else(|| {
                    if let Some(system_pwd) = get_full_path_name_w(&format!("{}:", drive)) {
                        self.map[index] = Some(system_pwd.clone());
                        system_pwd
                    } else {
                        format!(r"{}:\", drive)
                    }
                }))
            } else {
                None
            }
        }

        /// Expand a relative path using the PWD-per-drive, return PathBuf
        /// of absolute path.
        /// Return None if path is not valid or can't get drive letter.
        pub fn expand_path(&mut self, path: &Path) -> Option<PathBuf> {
            let path_str = path.to_str()?;
            if let Some(drive_letter) = Self::extract_drive_letter(path) {
                if let Some(pwd) = self.get_pwd(drive_letter) {
                    // Combine current PWD with the relative path
                    let mut base = PathBuf::from(Self::ensure_trailing_separator(&pwd));
                    base.push(path_str.split_at(2).1); // Skip the "C:" part of the relative path
                    return Some(base);
                }
            }
            None // Invalid path or has no drive letter
        }

        /// Extract the drive letter from a path (e.g., `C:test` -> `C`)
        fn extract_drive_letter(path: &Path) -> Option<char> {
            Some(
                path.to_str()
                    .and_then(|s| s.chars().next())
                    .filter(|c| c.is_ascii_alphabetic())?,
            )
        }

        /// Ensure a path has a trailing `\`
        fn ensure_trailing_separator(path: &str) -> String {
            if !path.ends_with('\\') && !path.ends_with('/') {
                format!(r"{}\", path)
            } else {
                path.to_string()
            }
        }
    }

    fn get_full_path_name_w(path_str: &str) -> Option<String> {
        use omnipath::sys_absolute;
        if let Ok(path_sys_abs) = sys_absolute(PathBuf::from(path_str).as_path()) {
            Some(path_sys_abs.to_str()?.to_string())
        } else {
            None
        }
    }

    use std::sync::{Mutex, OnceLock};

    /// Global singleton instance of DrivePwdMap
    static DRIVE_PWD_MAP: OnceLock<Mutex<Drive2PWD>> = OnceLock::new();

    /// Access the singleton instance
    fn get_drive_pwd_map() -> &'static Mutex<Drive2PWD> {
        DRIVE_PWD_MAP.get_or_init(|| Mutex::new(Drive2PWD::new()))
    }

    /// Test for Drive2PWD map
    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_singleton_set_and_get_pwd() {
            // To avoid conflict with other test threads (on testing result),
            // use different drive set in multiple singleton tests
            let drive_pwd_map = get_drive_pwd_map();
            {
                let mut map = drive_pwd_map.lock().unwrap();

                // Set PWD for drive X
                assert!(map.set_pwd(Path::new(r"X:\Users\Example")).is_ok());
            }

            {
                let mut map = drive_pwd_map.lock().unwrap();

                // Get PWD for drive X
                assert_eq!(map.get_pwd('X'), Some(r"X:\Users\Example".to_string()));

                // Get PWD for drive E (not set, should return E:\) ???
                // 11-21-2024 happened to start nushell from drive E:,
                // run toolkit test 'toolkit check pr' then this test failed
                // since the singleton has its own state, so this type of test ('not set,
                // should return ...') must be more careful to avoid accidentally fail.
                if let Some(pwd_on_e) = get_full_path_name_w("E:") {
                    assert_eq!(map.get_pwd('E'), Some(pwd_on_e));
                } else {
                    assert_eq!(map.get_pwd('E'), Some(r"E:\".to_string()));
                }
            }
        }

        #[test]
        fn test_expand_path() {
            let mut drive_map = Drive2PWD::new();

            // Set PWD for drive C
            assert_eq!(drive_map.set_pwd(Path::new(r"C:\Users\Home")), Ok(()));

            // Expand a relative path
            let expanded = drive_map.expand_path(Path::new(r"C:test"));
            assert_eq!(expanded, Some(PathBuf::from(r"C:\Users\Home\test")));

            // Expand an absolute path
            let expanded = drive_map.expand_path(Path::new(r"C:\absolute\path"));
            assert_eq!(expanded, Some(PathBuf::from(r"C:\absolute\path")));

            // Expand with no drive letter
            let expanded = drive_map.expand_path(Path::new(r"\no_drive"));
            assert_eq!(expanded, None);

            // Expand with no PWD set for the drive
            let expanded = drive_map.expand_path(Path::new("D:test"));
            if let Some(pwd_on_d) = get_full_path_name_w("D:") {
                assert_eq!(
                    expanded,
                    Some(PathBuf::from(format!(
                        r"{}test",
                        Drive2PWD::ensure_trailing_separator(&pwd_on_d)
                    )))
                );
            } else {
                assert_eq!(expanded, Some(PathBuf::from(r"D:\test")));
            }
        }

        #[test]
        fn test_set_and_get_pwd() {
            let mut drive_map = Drive2PWD::new();

            // Set PWD for drive C
            assert!(drive_map.set_pwd(Path::new(r"C:\Users\Example")).is_ok());
            assert_eq!(
                drive_map.get_pwd('C'),
                Some(r"C:\Users\Example".to_string())
            );

            // Set PWD for drive D
            assert!(drive_map.set_pwd(Path::new(r"D:\Projects")).is_ok());
            assert_eq!(drive_map.get_pwd('D'), Some(r"D:\Projects".to_string()));

            // Get PWD for drive E (not set, should return E:\)
            // 11-21-2024 happened to start nushell from drive E:,
            // run toolkit test 'toolkit check pr' then this test failed
            // if a drive has not been set PWD, it will ask system to get
            // current directory, so this type of test ('not set, should
            // return ...') must be more careful to avoid accidentally fail.
            if let Some(pwd_on_e) = get_full_path_name_w("E:") {
                assert_eq!(drive_map.get_pwd('E'), Some(pwd_on_e));
            } else {
                assert_eq!(drive_map.get_pwd('E'), Some(r"E:\".to_string()));
            }
        }

        #[test]
        fn test_set_pwd_invalid_path() {
            let mut drive_map = Drive2PWD::new();

            // Invalid path (no drive letter)
            let result = drive_map.set_pwd(Path::new(r"\InvalidPath"));
            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), r"Invalid path: \InvalidPath");
        }

        #[test]
        fn test_get_pwd_invalid_drive() {
            let mut drive_map = Drive2PWD::new();

            // Get PWD for a drive not set (e.g., Z)
            assert_eq!(drive_map.get_pwd('Z'), Some(r"Z:\".to_string()));

            // Invalid drive letter (non-alphabetic)
            assert_eq!(drive_map.get_pwd('1'), None);
        }
    }
}
