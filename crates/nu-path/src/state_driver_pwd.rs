use std::path::{ Path, PathBuf };

pub struct DrivePwdMap {
    map: [Option<String>; 26], // Fixed-size array for A-Z
}

impl DrivePwdMap {
    pub fn new() -> Self {
        DrivePwdMap {
            map: Default::default(), // Initialize all to `None`
        }
    }

    /// Set the current working directory based on the drive letter in the path
    pub fn set_pwd(&mut self, path: &Path) -> Result<(), String> {
        if let Some(drive_letter) = Self::extract_drive_letter(path) {
            if let Some(index) = Self::drive_to_index(drive_letter) {
                let normalized = Self::normalize_path(path);
                self.map[index] = Some(normalized.to_string_lossy().into_owned());
                Ok(())
            } else {
                Err(format!("Invalid drive letter: {}", drive_letter))
            }
        } else {
            Err(format!("Invalid path: {}", path.display()))
        }
    }

    /// Get the current working directory for a drive letter
    pub fn get_pwd(&self, drive: char) -> Option<String> {
        Self::drive_to_index(drive).map(|index| {
            self.map[index]
                .clone()
                .unwrap_or_else(|| format!("{}:\\", drive.to_ascii_uppercase()))
        })
    }

    /// Expand a relative path using the current working directory of the drive
    pub fn expand_path(&self, path: &Path) -> Option<PathBuf> {
        let path_str = path.to_str()?;
        if let Some(drive_letter) = Self::extract_drive_letter(path) {
            let is_absolute = path_str.contains(":\\") || path_str.starts_with("\\");
            if is_absolute {
                // Already an absolute path
                Some(PathBuf::from(Self::ensure_trailing_separator(path_str)))
            } else if let Some(pwd) = self.get_pwd(drive_letter) {
                // Combine current PWD with the relative path
                let mut base = PathBuf::from(Self::ensure_trailing_separator(&pwd));
                base.push(path_str.split_at(2).1); // Skip the "C:" part of the relative path
                Some(base)
            } else {
                None // Drive letter not found
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

    /// Normalize a path by removing any trailing `\` or `/`
    fn normalize_path(path: &Path) -> PathBuf {
        let mut normalized = path.to_path_buf();
        while normalized.to_string_lossy().ends_with(&['\\', '/'][..]) {
            normalized.pop();
        }
        normalized
    }

    /// Ensure a path has a trailing `\`
    fn ensure_trailing_separator(path: &str) -> String {
        if !path.ends_with('\\') && !path.ends_with('/') {
            format!("{}\\", path)
        } else {
            path.to_string()
        }
    }
}

use once_cell::sync::Lazy;
use std::sync::Mutex;
//use nu_protocol::errors::shell_error::ShellError;

/// Global singleton instance of DrivePwdMap
static DRIVE_PWD_MAP: Lazy<Mutex<DrivePwdMap>> = Lazy::new(|| Mutex::new(DrivePwdMap::new()));

/// Public API to access the singleton instance
pub fn get_drive_pwd_map() -> &'static Mutex<DrivePwdMap> {
    &DRIVE_PWD_MAP
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_expand_path() {
        let mut drive_map = DrivePwdMap::new();

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
    fn test_set_and_get_pwd() {
        let mut drive_map = DrivePwdMap::new();

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
        let mut drive_map = DrivePwdMap::new();

        // Invalid path (no drive letter)
        let result = drive_map.set_pwd(Path::new("\\InvalidPath"));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid path: \\InvalidPath");
    }

    #[test]
    fn test_get_pwd_invalid_drive() {
        let drive_map = DrivePwdMap::new();

        // Get PWD for a drive not set (e.g., Z)
        assert_eq!(drive_map.get_pwd('Z'), Some("Z:\\".to_string()));

        // Invalid drive letter (non-alphabetic)
        assert_eq!(drive_map.get_pwd('1'), None);
    }

    #[test]
    fn test_drive_to_index() {
        // Valid drive letters
        assert_eq!(DrivePwdMap::drive_to_index('A'), Some(0));
        assert_eq!(DrivePwdMap::drive_to_index('Z'), Some(25));
        // Valid drive letters
        assert_eq!(DrivePwdMap::drive_to_index('a'), Some(0));
        assert_eq!(DrivePwdMap::drive_to_index('z'), Some(25));
        for i in 1..25 {
            assert_eq!(DrivePwdMap::drive_to_index(std::char::from_u32(('A' as usize + i) as u32).unwrap()), Some(i));
            assert_eq!(DrivePwdMap::drive_to_index(std::char::from_u32(('a' as usize + i) as u32).unwrap()), Some(i));
        }

        // Invalid drive letters
        assert_eq!(DrivePwdMap::drive_to_index('1'), None);
        assert_eq!(DrivePwdMap::drive_to_index('$'), None);
    }
}

pub mod current_directory_specific {
    use std::path::Path;

    #[cfg(target_os = "windows")]
    pub fn need_expand_current_directory_per_drive(path: &Path) -> bool {
        if let Some(path_str) = path.to_str() {
            let chars: Vec<char> = path_str.chars().collect();
            if chars.len() >= 2 {
                return chars[1] == ':' && (chars.len() == 2 || (chars[2] != '/' && chars[2] != '\\'));
            }
        }
        false
    }

    #[cfg(not(target_os = "windows"))]
    pub fn need_expand_current_directory(path: &Path) -> bool {
        false
    }

    #[cfg(target_os = "windows")]
    pub fn get_windows_absolute_path(path: &Path) -> Option<String> {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        use std::os::windows::ffi::OsStrExt;
        use winapi::um::fileapi::GetFullPathNameW;
        use winapi::um::winnt::WCHAR;

        const MAX_PATH : usize = 260;
        let mut buffer: [WCHAR; MAX_PATH] = [0; MAX_PATH];

        if let Some(path_str) = path.to_str() {
            unsafe {
                // Convert input to wide string.
                let wide_path: Vec<u16> = OsString::from(path_str).encode_wide().chain(Some(0)).collect();
                let length = GetFullPathNameW(
                    wide_path.as_ptr(),
                    buffer.len() as u32,
                    buffer.as_mut_ptr(),
                    std::ptr::null_mut(),
                );

                if length > 0 {
                    let abs_path = OsString::from_wide(&buffer[..length as usize]);
                    if let Some(abs_path_str) = abs_path.to_str() {
                        let abs_path_string = abs_path_str.to_string();
                        return Some(abs_path_string);
                    }
                }
            }
        }

        None
    }

    pub enum DrivePwdError {
        InvalidPath,
        SystemError,
    }
    #[cfg(not(target_os = "windows"))]
    fn get_windows_absolute_path(path: &Path) -> Option<String> {
        None
    }
    #[cfg(target_os = "windows")]
    pub fn set_current_directory_windows(path: &Path) -> Result<(), DrivePwdError> {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::System::Environment::SetCurrentDirectoryW;

        if let Some(path_str) = path.to_str() {
            unsafe {
                // Convert input to wide string.
                let wide_path: Vec<u16> = OsString::from(path_str).encode_wide().chain(Some(0)).collect();
                if SetCurrentDirectoryW(wide_path.as_ptr()) != 0 {
                    println!("Successfully changed the current directory to {}", path_str);
                    return Ok(())
                } else {
                    return
                        Err(DrivePwdError::SystemError)

                };
            }
        }
        Err(DrivePwdError::InvalidPath)
    }

    #[cfg(not(target_os = "windows"))]
    pub fn set_current_directory_windows(_path: &Path) -> Result<(), Error>{
        Ok(())
    }
}