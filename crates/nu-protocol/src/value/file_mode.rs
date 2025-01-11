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

    pub fn get(&self) -> u32 {
        let mut mode = 0;
        mode |= if self.user.read { 0o400 } else { 0 };
        mode |= if self.user.write { 0o200 } else { 0 };
        mode |= if self.user.execute { 0o100 } else { 0 };
        mode |= if self.group.read { 0o040 } else { 0 };
        mode |= if self.group.write { 0o020 } else { 0 };
        mode |= if self.group.execute { 0o010 } else { 0 };
        mode |= if self.other.read { 0o004 } else { 0 };
        mode |= if self.other.write { 0o002 } else { 0 };
        mode |= if self.other.execute { 0o001 } else { 0 };
        mode
    }

    pub fn get_mode_string(&self) -> String {
        let mut mode_string = String::new();
        mode_string.push(if self.user.read { 'r' } else { '-' });
        mode_string.push(if self.user.write { 'w' } else { '-' });
        mode_string.push(if self.user.execute { 'x' } else { '-' });
        mode_string.push(if self.group.read { 'r' } else { '-' });
        mode_string.push(if self.group.write { 'w' } else { '-' });
        mode_string.push(if self.group.execute { 'x' } else { '-' });
        mode_string.push(if self.other.read { 'r' } else { '-' });
        mode_string.push(if self.other.write { 'w' } else { '-' });
        mode_string.push(if self.other.execute { 'x' } else { '-' });
        mode_string
    }
}

impl From<u32> for FileMode {
    fn from(value: u32) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for FileMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.get_mode_string())
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
}
