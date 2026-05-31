#[cfg(unix)]
use nix::unistd::{AccessFlags, access};
#[cfg(any(windows, unix))]
use std::path::Path;

// The result of checking whether we have permission to cd to a directory
#[derive(Debug)]
pub enum PermissionResult {
    PermissionOk,
    PermissionDenied,
}

// TODO: Maybe we should use file_attributes() from https://doc.rust-lang.org/std/os/windows/fs/trait.MetadataExt.html
// More on that here: https://learn.microsoft.com/en-us/windows/win32/fileio/file-attribute-constants
#[cfg(windows)]
pub fn have_permission(dir: impl AsRef<Path>) -> PermissionResult {
    match dir.as_ref().read_dir() {
        Err(e) => {
            if matches!(e.kind(), std::io::ErrorKind::PermissionDenied) {
                PermissionResult::PermissionDenied
            } else {
                PermissionResult::PermissionOk
            }
        }
        Ok(_) => PermissionResult::PermissionOk,
    }
}

#[cfg(unix)]
/// Check that the process' user id has permissions to execute or
/// in the case of a directory traverse the particular directory
pub fn have_permission(dir: impl AsRef<Path>) -> PermissionResult {
    // We check permissions for real user id, but that's fine, because in
    // proper installations of nushell, effective UID (EUID) rarely differs
    // from real UID (RUID). We strongly advise against setting the setuid bit
    // on the nushell executable or shebang scripts starts with `#!/usr/bin/env nu` e.g.
    // Most Unix systems ignore setuid on shebang by default anyway.
    access(dir.as_ref(), AccessFlags::X_OK).into()
}

#[cfg(unix)]
pub mod users {
    use nix::unistd::{Gid, Group, Uid, User};

    pub fn get_user_by_uid(uid: Uid) -> Option<User> {
        User::from_uid(uid).ok().flatten()
    }

    pub fn get_group_by_gid(gid: Gid) -> Option<Group> {
        Group::from_gid(gid).ok().flatten()
    }

    pub fn get_current_uid() -> Uid {
        Uid::current()
    }

    pub fn get_current_gid() -> Gid {
        Gid::current()
    }

    #[cfg(not(any(target_os = "linux", target_os = "freebsd", target_os = "android")))]
    pub fn get_current_username() -> Option<String> {
        get_user_by_uid(get_current_uid()).map(|user| user.name)
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "android"))]
    pub fn current_user_groups() -> Option<Vec<Gid>> {
        if let Ok(mut groups) = nix::unistd::getgroups() {
            groups.sort_unstable_by_key(|id| id.as_raw());
            groups.dedup();
            Some(groups)
        } else {
            None
        }
    }

    /// Returns groups for a provided user name and primary group id.
    ///
    /// # libc functions used
    ///
    /// - [`getgrouplist`](https://docs.rs/libc/*/libc/fn.getgrouplist.html)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use users::get_user_groups;
    ///
    /// for group in get_user_groups("stevedore", 1001).expect("Error looking up groups") {
    ///     println!("User is a member of group #{group}");
    /// }
    /// ```
    #[cfg(not(any(target_os = "linux", target_os = "freebsd", target_os = "android")))]
    pub fn get_user_groups(username: &str, gid: Gid) -> Option<Vec<Gid>> {
        let ugids = uzers::get_user_groups(username, gid.as_raw())?;
        if ugids.is_empty() {
            None
        } else {
            let mut gids: Vec<Gid> = ugids
                .into_iter()
                .map(|g| Gid::from_raw(g.gid()))
                .filter_map(get_group_by_gid)
                .map(|group| group.gid)
                .collect();
            gids.sort_unstable_by_key(|g| g.as_raw());
            gids.dedup();
            Some(gids)
        }
    }
}

impl<T, E> From<Result<T, E>> for PermissionResult {
    fn from(value: Result<T, E>) -> Self {
        match value {
            Ok(_) => Self::PermissionOk,
            Err(_) => Self::PermissionDenied,
        }
    }
}
