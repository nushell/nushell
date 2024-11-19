cfg_if::cfg_if! {
    if #[cfg(target_os="windows")] {

use once_cell::sync::Lazy;
use std::path::{ Path, PathBuf };
use std::sync::Mutex;

struct DrivePWDmap {
    map: [Option<String>; 26], // Fixed-size array for A-Z
}

impl DrivePWDmap {
    pub fn new() -> Self {
        DrivePWDmap {
            map: Default::default(), // Initialize all to `None`
        }
    }

    /// Set the PWD for the drive letter in the path which is an absolute path
    pub fn set_pwd(&mut self, path: &Path) -> Result<(), String> {
        if let Some(drive_letter) = Self::extract_drive_letter(path) {
            if let Some(index) = Self::drive_to_index(drive_letter) {
                if let Some(path_str) = path.to_str() {
                    self.map[index] = Some(path_str.to_string());
                    Ok(())
                } else {
                    Err(format!("Invalid path: {}", path.display()))
                }
            } else {
                Err(format!("Invalid drive letter: {}", drive_letter))
            }
        } else {
            Err(format!("Invalid path: {}", path.display()))
        }
    }

    /// Get the PWD for a drive letter, if not yet, try using
    /// winapi GetFullPathNameW to get "T:", "T:/" can be default
    pub fn get_pwd(&mut self, drive: char) -> Option<String> {
        Self::drive_to_index(drive).map(|index| {
            self.map[index]
                .clone()
                .unwrap_or_else(||
                        if let Some(system_pwd) = get_full_path_name_w(&format!("{}:", drive.to_ascii_uppercase())) {
                            self.map[index] = Some(system_pwd.clone());
                            system_pwd
                        } else {
                            format!("{}:/", drive.to_ascii_uppercase())
                        }
                    )
        })
    }

    /// Expand a relative path using the PWD of the drive
    pub fn expand_path(&mut self, path: &Path) -> Option<PathBuf> {
        let path_str = path.to_str()?;
        if let Some(drive_letter) = Self::extract_drive_letter(path) {
            if let Some(pwd) = self.get_pwd(drive_letter) {
                // Combine current PWD with the relative path
                let mut base = PathBuf::from(Self::ensure_trailing_separator(&pwd));
                base.push(path_str.split_at(2).1); // Skip the "C:" part of the relative path
                Some(base)
            } else {
                None // PWD on Drive letter not found
            }
        } else {
            None // Invalid or no drive letter
        }
    }

    /// Helper to convert a drive letter to an array index
    fn drive_to_index(drive: char) -> Option<usize> {
        let drive = drive.to_ascii_uppercase();
        if ('A'..='Z').contains(&drive) {
            Some((drive as usize) - ('A' as usize))
        } else {
            None
        }
    }

    /// Extract the drive letter from a path (e.g., `C:test` -> `C`)
    fn extract_drive_letter(path: &Path) -> Option<char> {
        path.to_str()
            .and_then(|s| s.chars().next())
            .filter(|c| c.is_ascii_alphabetic())
    }

    /// Ensure a path has a trailing `\`
    fn ensure_trailing_separator(path: &str) -> String {
        if !path.ends_with('\\') && !path.ends_with('/') {
            format!("{}/", path)
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
    use winapi::um::winnt::WCHAR;

    const MAX_PATH : usize = 260;
    let mut buffer: [WCHAR; MAX_PATH] = [0; MAX_PATH];

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
static DRIVE_PWD_MAP: Lazy<Mutex<DrivePWDmap>> = Lazy::new(|| Mutex::new(DrivePWDmap::new()));

/// Public API to access the singleton instance
fn get_drive_pwd_map() -> &'static Mutex<DrivePWDmap> {
    &DRIVE_PWD_MAP
}

/// Test for DrivePWD map
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_singleton_set_and_get_pwd() {
        let drive_pwd_map = get_drive_pwd_map();
        {
            let mut map = drive_pwd_map.lock().unwrap();

            // Set PWD for drive C
            assert!(map.set_pwd(Path::new("C:\\Users\\Example")).is_ok());
        }

        {
            let map = drive_pwd_map.lock().unwrap();

            // Get PWD for drive C
            assert_eq!(map.get_pwd('C'), Some("C:\\Users\\Example".to_string()));

            // Get PWD for drive E (not set, should return E:\)
            assert_eq!(map.get_pwd('E'), Some("E:\\".to_string()));
        }
    }
    #[test]
    fn test_expand_path() {
        let mut drive_map = DrivePWDmap::new();

        // Set PWD for drive C
        drive_map.set_pwd(Path::new("C:\\Users\\Home")).unwrap();

        // Expand a relative path
        let expanded = drive_map.expand_path(Path::new("C:test"));
        assert_eq!(expanded, Some(PathBuf::from("C:\\Users\\Home\\test")));

        // Expand an absolute path
        let expanded = drive_map.expand_path(Path::new("C:\\absolute\\path"));
        assert_eq!(expanded, Some(PathBuf::from("C:\\absolute\\path")));

        // Expand with no drive letter
        let expanded = drive_map.expand_path(Path::new("\\no_drive"));
        assert_eq!(expanded, None);

        // Expand with no PWD set for the drive
        let expanded = drive_map.expand_path(Path::new("D:test"));
        assert_eq!(expanded, Some(PathBuf::from("D:\\test")));
    }

    #[test]
    fn test_set_and_get_pwd() {
        let mut drive_map = DrivePWDmap::new();

        // Set PWD for drive C
        assert!(drive_map.set_pwd(Path::new("C:\\Users\\Example")).is_ok());
        assert_eq!(drive_map.get_pwd('C'), Some("C:\\Users\\Example".to_string()));

        // Set PWD for drive D
        assert!(drive_map.set_pwd(Path::new("D:\\Projects")).is_ok());
        assert_eq!(drive_map.get_pwd('D'), Some("D:\\Projects".to_string()));

        // Get PWD for drive E (not set, should return E:\)
        assert_eq!(drive_map.get_pwd('E'), Some("E:\\".to_string()));
    }

    #[test]
    fn test_set_pwd_invalid_path() {
        let mut drive_map = DrivePWDmap::new();

        // Invalid path (no drive letter)
        let result = drive_map.set_pwd(Path::new("\\InvalidPath"));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid path: \\InvalidPath");
    }

    #[test]
    fn test_get_pwd_invalid_drive() {
        let drive_map = DrivePWDmap::new();

        // Get PWD for a drive not set (e.g., Z)
        assert_eq!(drive_map.get_pwd('Z'), Some("Z:\\".to_string()));

        // Invalid drive letter (non-alphabetic)
        assert_eq!(drive_map.get_pwd('1'), None);
    }

    #[test]
    fn test_drive_to_index() {
        // Valid drive letters
        assert_eq!(DrivePWDmap::drive_to_index('A'), Some(0));
        assert_eq!(DrivePWDmap::drive_to_index('Z'), Some(25));
        // Valid drive letters
        assert_eq!(DrivePWDmap::drive_to_index('a'), Some(0));
        assert_eq!(DrivePWDmap::drive_to_index('z'), Some(25));
        for i in 1..25 {
            assert_eq!(DrivePWDmap::drive_to_index(std::char::from_u32(('A' as usize + i) as u32).unwrap()), Some(i));
            assert_eq!(DrivePWDmap::drive_to_index(std::char::from_u32(('a' as usize + i) as u32).unwrap()), Some(i));
        }

        // Invalid drive letters
        assert_eq!(DrivePWDmap::drive_to_index('1'), None);
        assert_eq!(DrivePWDmap::drive_to_index('$'), None);
    }
}}}

/// Usage for pwd_per_drive
///
/// Upon change PWD, call set_pwd_per_drive() with absolute path
///
/// Call expand_pwd_per_drive() with relative path to get absolution path
///
/// Doctest
/// ```Rust
///         // Set PWD for drive C
///         set_pwd_per_drive(Path::new("C:\\Users\\Home")).unwrap();
///
///         // Expand a relative path
///         let expanded = expand_pwd_per_drive(Path::new("C:test"));
///         assert_eq!(expanded, Some(PathBuf::from("C:\\Users\\Home\\test")));
///
///         // Will NOT expand an absolute path
///         let expanded = expand_pwd_per_drive(Path::new("C:\\absolute\\path"));
///         assert_eq!(expanded, None);
///
///         // Expand with no drive letter
///         let expanded = expand_pwd_per_drive(Path::new("\\no_drive"));
///         assert_eq!(expanded, None);
///
///         // Expand with no PWD set for the drive
///         let expanded = expand_pwd_per_drive(Path::new("D:test"));
///         assert_eq!(expanded, Some(PathBuf::from("D:\\test")));
/// ```
pub mod pwd_per_drive {
    use std::path::{ Path, PathBuf };
    use super::{get_drive_pwd_map};

    /// set_pwd_per_drive
    /// record PWD for drive, path must be absolute path
    /// return Ok(()) if succeeded, otherwise error message
    #[cfg(target_os = "windows")]
    pub fn set_pwd_per_drive(path: &Path) -> Result<(), String> {
        get_drive_pwd_map().lock().unwrap().set_pwd(path)
    }

    #[cfg(not(target_os = "windows"))]
    pub fn set_pwd_per_drive(path: &Path) -> Result<(), String> {
        Ok(())
    }

    /// expand_pwe_per_drive
    /// input relative path, expand PWD to construct absolute path
    /// return PathBuf for absolute path, None if input path is invalid
    #[cfg(target_os = "windows")]
    pub fn expand_pwd_per_drive(path: &Path) -> Option<PathBuf> {
        if need_expand_pwd_per_drive(path) {
            get_drive_pwd_map().lock().unwrap().expand_path(path)
        } else {
            None
        }
    }

    /// expand_pwd_per_drive will return None on non-windows platform
    #[cfg(not(target_os = "windows"))]
    pub fn expand_pwd_per_drive(_path: &Path) -> Option<PathBuf> {
        None
    }

    /// If input path is relative path with drive letter,
    /// it can be expanded with PWD per drive
    #[cfg(target_os = "windows")]
    fn need_expand_pwd_per_drive(path: &Path) -> bool {
        if let Some(path_str) = path.to_str() {
            let chars: Vec<char> = path_str.chars().collect();
            if chars.len() >= 2 {
                return chars[1] == ':' && (chars.len() == 2 || (chars[2] != '/' && chars[2] != '\\'));
            }
        }
        false
    }

    /// On non-windows platform, will not expand
    #[cfg(not(target_os = "windows"))]
    fn need_expand_pwd_per_drive(path: &Path) -> bool {
        false
    }

    #[test]
    fn test_usage_for_pwd_per_drive() {
        // Set PWD for drive C
        set_pwd_per_drive(Path::new("C:\\Users\\Home")).unwrap();

        // Expand a relative path
        let expanded = expand_pwd_per_drive(Path::new("C:test"));
        assert_eq!(expanded, Some(PathBuf::from("C:\\Users\\Home\\test")));

        // Will NOT expand an absolute path
        let expanded = expand_pwd_per_drive(Path::new("C:\\absolute\\path"));
        assert_eq!(expanded, None);

        // Expand with no drive letter
        let expanded = expand_pwd_per_drive(Path::new("\\no_drive"));
        assert_eq!(expanded, None);

        // Expand with no PWD set for the drive
        let expanded = expand_pwd_per_drive(Path::new("D:test"));
        assert_eq!(expanded, Some(PathBuf::from("D:\\test")));
    }
}