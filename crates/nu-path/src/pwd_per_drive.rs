/// Usage for pwd_per_drive on windows
///
/// let mut map = DriveToPwdMap::new();
///
/// Upon change PWD, call map.set_pwd() with absolute path
///
/// Call map.expand_pwd() with relative path to get absolution path
///
/// ```
/// use std::path::{Path, PathBuf};
/// use nu_path::DriveToPwdMap;
///
/// let mut map = DriveToPwdMap::new();
///
/// // Set PWD for drive C
/// assert!(map.set_pwd(Path::new(r"C:\Users\Home")).is_ok());
///
/// // Expand a relative path
/// let expanded = map.expand_pwd(Path::new("c:test"));
/// assert_eq!(expanded, Some(PathBuf::from(r"C:\Users\Home\test")));
///
/// // Will NOT expand an absolute path
/// let expanded = map.expand_pwd(Path::new(r"C:\absolute\path"));
/// assert_eq!(expanded, None);
///
/// // Expand with no drive letter
/// let expanded = map.expand_pwd(Path::new(r"\no_drive"));
/// assert_eq!(expanded, None);
///
/// // Expand with no PWD set for the drive
/// let expanded = map.expand_pwd(Path::new("D:test"));
/// assert!(expanded.is_some());
/// let abs_path = expanded.unwrap().as_path().to_str().expect("OK").to_string();
/// assert!(abs_path.starts_with(r"D:\"));
/// assert!(abs_path.ends_with(r"\test"));
///
/// // Get env vars for child process
/// use std::collections::HashMap;
/// let mut env = HashMap::<String, String>::new();
/// map.get_env_vars(&mut env);
/// assert_eq!(env.get("=C:").unwrap(), r"C:\Users\Home");
/// ```
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
pub enum PathError {
    InvalidDriveLetter,
    InvalidPath,
}

/// Helper to check if input path is relative path
/// with drive letter, it can be expanded with PWD-per-drive.
fn need_expand(path: &Path) -> bool {
    if let Some(path_str) = path.to_str() {
        let chars: Vec<char> = path_str.chars().collect();
        if chars.len() >= 2 {
            return chars[1] == ':' && (chars.len() == 2 || (chars[2] != '/' && chars[2] != '\\'));
        }
    }
    false
}

#[derive(Clone, Debug)]
pub struct DriveToPwdMap {
    map: [Option<String>; 26], // Fixed-size array for A-Z
}

impl Default for DriveToPwdMap {
    fn default() -> Self {
        Self::new()
    }
}

impl DriveToPwdMap {
    pub fn new() -> Self {
        Self {
            map: Default::default(),
        }
    }

    pub fn env_var_for_drive(drive_letter: char) -> String {
        let drive_letter = drive_letter.to_ascii_uppercase();
        format!("={}:", drive_letter)
    }

    /// Collect PWD-per-drive as env vars (for child process)
    pub fn get_env_vars(&self, env: &mut HashMap<String, String>) {
        for (drive_index, drive_letter) in ('A'..='Z').enumerate() {
            if let Some(pwd) = self.map[drive_index].clone() {
                if pwd.len() > 3 {
                    let env_var_for_drive = Self::env_var_for_drive(drive_letter);
                    env.insert(env_var_for_drive, pwd);
                }
            }
        }
    }

    /// Set the PWD for the drive letter in the absolute path.
    /// Return PathError for error.
    pub fn set_pwd(&mut self, path: &Path) -> Result<(), PathError> {
        if let (Some(drive_letter), Some(path_str)) =
            (Self::extract_drive_letter(path), path.to_str())
        {
            if drive_letter.is_ascii_alphabetic() {
                let drive_letter = drive_letter.to_ascii_uppercase();
                // Make sure saved drive letter is upper case
                let mut c = path_str.chars();
                match c.next() {
                    None => Err(PathError::InvalidDriveLetter),
                    Some(_) => {
                        let drive_index = drive_letter as usize - 'A' as usize;
                        let normalized_pwd = drive_letter.to_string() + c.as_str();
                        self.map[drive_index] = Some(normalized_pwd);
                        Ok(())
                    }
                }
            } else {
                Err(PathError::InvalidDriveLetter)
            }
        } else {
            Err(PathError::InvalidPath)
        }
    }

    /// Get the PWD for drive, if not yet, ask GetFullPathNameW() or omnipath,
    /// or else return default r"X:\".
    fn get_pwd(&self, drive_letter: char) -> Result<String, PathError> {
        if drive_letter.is_ascii_alphabetic() {
            let drive_letter = drive_letter.to_ascii_uppercase();
            let drive_index = drive_letter as usize - 'A' as usize;
            Ok(self.map[drive_index].clone().unwrap_or_else(|| {
                if let Some(sys_pwd) = get_full_path_name_w(&format!("{}:", drive_letter)) {
                    sys_pwd
                } else {
                    format!(r"{}:\", drive_letter)
                }
            }))
        } else {
            Err(PathError::InvalidDriveLetter)
        }
    }

    /// Expand a relative path using the PWD-per-drive, return PathBuf
    /// of absolute path.
    /// Return None if path is not valid or can't get drive letter.
    pub fn expand_pwd(&self, path: &Path) -> Option<PathBuf> {
        if need_expand(path) {
            let path_str = path.to_str()?;
            if let Some(drive_letter) = Self::extract_drive_letter(path) {
                if let Ok(pwd) = self.get_pwd(drive_letter) {
                    // Combine current PWD with the relative path
                    let mut base = PathBuf::from(Self::ensure_trailing_delimiter(&pwd));
                    // need_expand() and extract_drive_letter() all ensure path_str.len() >= 2
                    base.push(&path_str[2..]); // Join PWD with path parts after "C:"
                    return Some(base);
                }
            }
        }
        None // Invalid path or has no drive letter
    }

    /// Helper to extract the drive letter from a path, keep case
    ///  (e.g., `C:test` -> `C`, `d:\temp` -> `d`)
    fn extract_drive_letter(path: &Path) -> Option<char> {
        path.to_str()
            .and_then(|s| s.chars().next())
            .filter(|c| c.is_ascii_alphabetic())
    }

    /// Ensure a path has a trailing `\\` or '/'
    fn ensure_trailing_delimiter(path: &str) -> String {
        if !path.ends_with('\\') && !path.ends_with('/') {
            format!(r"{}\", path)
        } else {
            path.to_string()
        }
    }
}

fn get_full_path_name_w(path_str: &str) -> Option<String> {
    use omnipath::sys_absolute;
    if let Ok(path_sys_abs) = sys_absolute(Path::new(path_str)) {
        Some(path_sys_abs.to_str()?.to_string())
    } else {
        None
    }
}

/// Test for Drive2PWD map
#[cfg(test)]
mod tests {
    use super::*;

    /// Test or demo usage of PWD-per-drive
    /// In doctest, there's no get_full_path_name_w available so can't foresee
    /// possible result, here can have more accurate test assert
    #[test]
    fn test_usage_for_pwd_per_drive() {
        let mut map = DriveToPwdMap::new();

        // Set PWD for drive E
        assert!(map.set_pwd(Path::new(r"E:\Users\Home")).is_ok());

        // Expand a relative path
        let expanded = map.expand_pwd(Path::new("e:test"));
        assert_eq!(expanded, Some(PathBuf::from(r"E:\Users\Home\test")));

        // Will NOT expand an absolute path
        let expanded = map.expand_pwd(Path::new(r"E:\absolute\path"));
        assert_eq!(expanded, None);

        // Expand with no drive letter
        let expanded = map.expand_pwd(Path::new(r"\no_drive"));
        assert_eq!(expanded, None);

        // Expand with no PWD set for the drive
        let expanded = map.expand_pwd(Path::new("F:test"));
        if let Some(sys_abs) = get_full_path_name_w("F:") {
            assert_eq!(
                expanded,
                Some(PathBuf::from(format!(
                    "{}test",
                    DriveToPwdMap::ensure_trailing_delimiter(&sys_abs)
                )))
            );
        } else {
            assert_eq!(expanded, Some(PathBuf::from(r"F:\test")));
        }
    }

    #[test]
    fn test_get_env_vars() {
        let mut map = DriveToPwdMap::new();
        map.set_pwd(Path::new(r"I:\Home")).unwrap();
        map.set_pwd(Path::new(r"j:\User")).unwrap();

        let mut env = HashMap::<String, String>::new();
        map.get_env_vars(&mut env);
        assert_eq!(
            env.get(&DriveToPwdMap::env_var_for_drive('I')).unwrap(),
            r"I:\Home"
        );
        assert_eq!(
            env.get(&DriveToPwdMap::env_var_for_drive('J')).unwrap(),
            r"J:\User"
        );
    }

    #[test]
    fn test_expand_pwd() {
        let mut drive_map = DriveToPwdMap::new();

        // Set PWD for drive 'M:'
        assert_eq!(drive_map.set_pwd(Path::new(r"M:\Users")), Ok(()));
        // or 'm:'
        assert_eq!(drive_map.set_pwd(Path::new(r"m:\Users\Home")), Ok(()));

        // Expand a relative path on "M:"
        let expanded = drive_map.expand_pwd(Path::new(r"M:test"));
        assert_eq!(expanded, Some(PathBuf::from(r"M:\Users\Home\test")));
        // or on "m:"
        let expanded = drive_map.expand_pwd(Path::new(r"m:test"));
        assert_eq!(expanded, Some(PathBuf::from(r"M:\Users\Home\test")));

        // Expand an absolute path
        let expanded = drive_map.expand_pwd(Path::new(r"m:\absolute\path"));
        assert_eq!(expanded, None);

        // Expand with no drive letter
        let expanded = drive_map.expand_pwd(Path::new(r"\no_drive"));
        assert_eq!(expanded, None);

        // Expand with no PWD set for the drive
        let expanded = drive_map.expand_pwd(Path::new("N:test"));
        if let Some(pwd_on_drive) = get_full_path_name_w("N:") {
            assert_eq!(
                expanded,
                Some(PathBuf::from(format!(
                    r"{}test",
                    DriveToPwdMap::ensure_trailing_delimiter(&pwd_on_drive)
                )))
            );
        } else {
            assert_eq!(expanded, Some(PathBuf::from(r"N:\test")));
        }
    }

    #[test]
    fn test_set_and_get_pwd() {
        let mut drive_map = DriveToPwdMap::new();

        // Set PWD for drive 'O'
        assert!(drive_map.set_pwd(Path::new(r"O:\Users")).is_ok());
        // Or for drive 'o'
        assert!(drive_map.set_pwd(Path::new(r"o:\Users\Example")).is_ok());
        // Get PWD for drive 'O'
        assert_eq!(drive_map.get_pwd('O'), Ok(r"O:\Users\Example".to_string()));
        // or 'o'
        assert_eq!(drive_map.get_pwd('o'), Ok(r"O:\Users\Example".to_string()));

        // Get PWD for drive P (not set yet, but system might already
        // have PWD on this drive)
        if let Some(pwd_on_drive) = get_full_path_name_w("P:") {
            assert_eq!(drive_map.get_pwd('P'), Ok(pwd_on_drive));
        } else {
            assert_eq!(drive_map.get_pwd('P'), Ok(r"P:\".to_string()));
        }
    }

    #[test]
    fn test_set_pwd_invalid_path() {
        let mut drive_map = DriveToPwdMap::new();

        // Invalid path (no drive letter)
        let result = drive_map.set_pwd(Path::new(r"\InvalidPath"));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), PathError::InvalidPath);
    }

    #[test]
    fn test_get_pwd_invalid_drive() {
        let drive_map = DriveToPwdMap::new();

        // Get PWD for a drive not set (e.g., Z)
        assert_eq!(drive_map.get_pwd('Z'), Ok(r"Z:\".to_string()));

        // Invalid drive letter (non-alphabetic)
        assert_eq!(drive_map.get_pwd('1'), Err(PathError::InvalidDriveLetter));
    }
}
