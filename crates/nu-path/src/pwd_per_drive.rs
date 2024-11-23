use std::path::{Path, PathBuf};

cfg_if::cfg_if! { if #[cfg(windows)] {


struct Drive2PWDmap {
    map: [Option<String>; 26], // Fixed-size array for A-Z
}

impl Drive2PWDmap {
    pub fn new() -> Self {
        Drive2PWDmap {
            map: Default::default(), // Initialize all to `None`
        }
    }

    /// Set the PWD for the drive letter in the absolute path.
    /// Return String for error description.
    pub fn set_pwd(&mut self, path: &Path) -> Result<(), String> {
        if let (Some(drive_letter), Some(path_str)) = (
            Self::extract_drive_letter(path),
            path.to_str(),
        ) {
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
            Some(self.map[index]
                .clone()
                .unwrap_or_else(||
                    if let Some(system_pwd) = get_full_path_name_w(&format!("{}:", drive)) {
                        self.map[index] = Some(system_pwd.clone());
                        system_pwd
                    } else {
                        format!(r"{}:\", drive)
                    }
                )
            )
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
                return Some(base)
            }
        }
        None // Invalid path or has no drive letter
    }

    /// Extract the drive letter from a path (e.g., `C:test` -> `C`)
    fn extract_drive_letter(path: &Path) -> Option<char> {
        Some(path.to_str()
            .and_then(|s| s.chars().next())
            .filter(|c| c.is_ascii_alphabetic())?
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

// GetFullPathW
fn get_full_path_name_w(path_str: &str) -> Option<String> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::os::windows::ffi::OsStrExt;
    use winapi::um::fileapi::GetFullPathNameW;

    const MAX_PATH : usize = 260;
    let mut buffer: [u16; MAX_PATH] = [0; MAX_PATH];

    unsafe {
        // Convert input to wide string.
        let wide_path: Vec<u16> = OsString::from(path_str).encode_wide().chain(Some(0)).collect();
        let length = GetFullPathNameW(
            wide_path.as_ptr(),
            buffer.len() as u32,
            buffer.as_mut_ptr(),
            std::ptr::null_mut(),
        );

        if length > 0 && (length as usize) < MAX_PATH {
            let path = OsString::from_wide(&buffer[..length as usize]);
            if let Some(path_str) = path.to_str() {
                let path_string = path_str.to_string();
                {
                    return Some(path_string);
                }
            }
        }
    }
    None
}

/// Global singleton instance of DrivePwdMap
use std::sync::{Once, Mutex};

static INIT: Once = Once::new();
static mut DRIVE_PWD_MAP: Option<Mutex<Drive2PWDmap>> = None;

/// Access the singleton instance
fn get_drive_pwd_map() -> &'static Mutex<Drive2PWDmap> {
    unsafe {
        INIT.call_once(|| {
            DRIVE_PWD_MAP = Some(Mutex::new(Drive2PWDmap::new()));
        });
        DRIVE_PWD_MAP.as_ref().unwrap()
    }
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
        let mut drive_map = Drive2PWDmap::new();

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
            assert_eq!(expanded, Some(PathBuf::from(format!(r"{}test", Drive2PWDmap::ensure_trailing_separator(&pwd_on_d)))));
        } else {
            assert_eq!(expanded, Some(PathBuf::from(r"D:\test")));
        }
    }

    #[test]
    fn test_set_and_get_pwd() {
        let mut drive_map = Drive2PWDmap::new();

        // Set PWD for drive C
        assert!(drive_map.set_pwd(Path::new(r"C:\Users\Example")).is_ok());
        assert_eq!(drive_map.get_pwd('C'), Some(r"C:\Users\Example".to_string()));

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
        let mut drive_map = Drive2PWDmap::new();

        // Invalid path (no drive letter)
        let result = drive_map.set_pwd(Path::new(r"\InvalidPath"));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), r"Invalid path: \InvalidPath");
    }

    #[test]
    fn test_get_pwd_invalid_drive() {
        let mut drive_map = Drive2PWDmap::new();

        // Get PWD for a drive not set (e.g., Z)
        assert_eq!(drive_map.get_pwd('Z'), Some(r"Z:\".to_string()));

        // Invalid drive letter (non-alphabetic)
        assert_eq!(drive_map.get_pwd('1'), None);
    }
}

}} // cfg_if! if #[cfg(windows)]

/// Usage for pwd_per_drive on windows
///
/// Upon change PWD, call set_pwd_per_drive() with absolute path
///
/// Call expand_pwd_per_drive() with relative path to get absolution path
///
/// ```
/// use std::path::{Path, PathBuf};
/// use nu_path::{expand_pwd_per_drive, set_pwd_per_drive};
///
/// //assert!(false); // Comment out to verify really tested
/// if cfg!(windows) {
///     // Set PWD for drive C
///     set_pwd_per_drive(Path::new(r"C:\Users\Home")).unwrap();
///
///     // Expand a relative path
///     let expanded = expand_pwd_per_drive(Path::new("C:test"));
///     assert_eq!(expanded, Some(PathBuf::from(r"C:\Users\Home\test")));
///
///     // Will NOT expand an absolute path
///     let expanded = expand_pwd_per_drive(Path::new(r"C:\absolute\path"));
///     assert_eq!(expanded, None);
///
///     // Expand with no drive letter
///     let expanded = expand_pwd_per_drive(Path::new(r"\no_drive"));
///     assert_eq!(expanded, None);
///
///     // Expand with no PWD set for the drive
///     let expanded = expand_pwd_per_drive(Path::new("D:test"));
///     assert_eq!(expanded, Some(PathBuf::from(r"D:\test")));
/// }
/// ```
pub mod pwd_per_drive_singleton {
    use super::*;

    /// set_pwd_per_drive
    /// On Windows, record PWD for drive, path must be absolute path
    /// return Ok(()) if succeeded, otherwise error message
    /// Other platforms, return Ok(())
    pub fn set_pwd_per_drive(_path: &Path) -> Result<(), String> {
        cfg_if::cfg_if! { if #[cfg(target_os="windows")] {

            if let Ok(mut pwd_per_drive) = get_drive_pwd_map().lock() {
                pwd_per_drive.set_pwd(_path)
            } else {
                Err("Failed to lock map".to_string())
            }

        } else {

            Ok(())

        }}
    }

    /// expand_pwe_per_drive
    /// On windows, input relative path, expand PWD-per-drive to construct absolute path
    /// return PathBuf for absolute path, None if input path is invalid.
    /// Otherwise, return None.
    pub fn expand_pwd_per_drive(_path: &Path) -> Option<PathBuf> {
        cfg_if::cfg_if! { if #[cfg(target_os="windows")] {

        if need_expand_pwd_per_drive(_path) {
            if let Ok(mut pwd_per_drive) = get_drive_pwd_map().lock() {
                return pwd_per_drive.expand_path(_path);
            }
        }

        }}

        None
    }

    cfg_if::cfg_if! { if #[cfg(target_os="windows")] {
    /// Helper only used on Windows, if input path is relative path
    /// with drive letter, it can be expanded with PWD-per-drive.
    fn need_expand_pwd_per_drive(_path: &Path) -> bool {
        if let Some(path_str) = _path.to_str() {
            let chars: Vec<char> = path_str.chars().collect();
            if chars.len() >= 2 {
                return chars[1] == ':' &&
                        (chars.len() == 2 ||
                            (chars[2] != '/' &&
                             chars[2] != '\\'
                            )
                        );
            }
        }
        false
    }
    }}

    #[test]
    fn test_usage_for_pwd_per_drive() {
        // Set PWD for drive F
        assert!(set_pwd_per_drive(Path::new(r"F:\Users\Home")).is_ok());

        if cfg!(windows) {
            // Expand a relative path
            let expanded = expand_pwd_per_drive(Path::new("f:test"));
            assert_eq!(expanded, Some(PathBuf::from(r"F:\Users\Home\test")));

            // Will NOT expand an absolute path
            let expanded = expand_pwd_per_drive(Path::new(r"F:\absolute\path"));
            assert_eq!(expanded, None);

            // Expand with no drive letter
            let expanded = expand_pwd_per_drive(Path::new(r"\no_drive"));
            assert_eq!(expanded, None);

            // Expand with no PWD set for the drive
            let expanded = expand_pwd_per_drive(Path::new("G:test"));
            assert_eq!(expanded, Some(PathBuf::from(r"G:\test")));
        } else {
            // None always
            assert_eq!(None, expand_pwd_per_drive(Path::new("F:test")));
        }
    }
}
