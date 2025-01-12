use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FileMode {
    user: FilePermission,
    group: FilePermission,
    other: FilePermission,
}

impl FileMode {
    pub const fn new(mode: u32) -> Self {
        let user_perm =
            FilePermission::new(mode & 0o400 != 0, mode & 0o200 != 0, mode & 0o100 != 0);
        let group_perm =
            FilePermission::new(mode & 0o040 != 0, mode & 0o020 != 0, mode & 0o010 != 0);
        let other_perm =
            FilePermission::new(mode & 0o004 != 0, mode & 0o002 != 0, mode & 0o001 != 0);
        Self {
            user: user_perm,
            group: group_perm,
            other: other_perm,
        }
    }

    pub const fn get_by_index(&self, index: usize) -> Option<FilePermission> {
        match index {
            0 => Some(self.user),
            1 => Some(self.group),
            2 => Some(self.other),
            _ => None,
        }
    }

    pub fn get_by_name(&self, name: &str) -> Option<FilePermission> {
        match name {
            "user" => Some(self.user),
            "group" => Some(self.group),
            "other" => Some(self.other),
            _ => None,
        }
    }
}

impl From<u32> for FileMode {
    fn from(value: u32) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for FileMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut mode_string = String::new();
        mode_string.push_str(&self.user.to_string());
        mode_string.push_str(&self.group.to_string());
        mode_string.push_str(&self.other.to_string());
        write!(f, "{}", mode_string)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FilePermission {
    read: bool,
    write: bool,
    execute: bool,
}

impl FilePermission {
    pub const fn new(read: bool, write: bool, execute: bool) -> Self {
        Self {
            read,
            write,
            execute,
        }
    }

    pub const fn get_by_index(&self, index: usize) -> Option<bool> {
        match index {
            0 => Some(self.read),
            1 => Some(self.write),
            2 => Some(self.execute),
            _ => None,
        }
    }

    pub fn get_by_name(&self, name: &str) -> Option<bool> {
        match name {
            "read" => Some(self.read),
            "write" => Some(self.write),
            "execute" => Some(self.execute),
            _ => None,
        }
    }
}

impl fmt::Display for FilePermission {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut mode_string = String::new();
        mode_string.push(if self.read { 'r' } else { '-' });
        mode_string.push(if self.write { 'w' } else { '-' });
        mode_string.push(if self.execute { 'x' } else { '-' });
        write!(f, "{}", mode_string)
    }
}

#[cfg(test)]
mod tests {
    use super::{FileMode, FilePermission};

    #[test]
    fn test_file_mode() {
        let mode = FileMode::new(0o755);
        assert_eq!(mode.to_string(), "rwxr-xr-x");
    }

    #[test]
    fn test_file_permission() {
        let permission = FilePermission::new(true, false, true);
        assert_eq!(permission.to_string(), "r-x");
    }
}
