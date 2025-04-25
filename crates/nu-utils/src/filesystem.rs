#[cfg(unix)]
use nix::unistd::{access, AccessFlags};
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
        use nix::libc::{c_int, gid_t};
        use std::ffi::CString;

        // MacOS uses i32 instead of gid_t in getgrouplist for unknown reasons
        #[cfg(target_os = "macos")]
        let mut buff: Vec<i32> = vec![0; 1024];
        #[cfg(not(target_os = "macos"))]
        let mut buff: Vec<gid_t> = vec![0; 1024];

        let name = CString::new(username).ok()?;

        let mut count = buff.len() as c_int;

        // MacOS uses i32 instead of gid_t in getgrouplist for unknown reasons
        // SAFETY:
        // int getgrouplist(const char *user, gid_t group, gid_t *groups, int *ngroups);
        //
        // `name` is valid CStr to be `const char*` for `user`
        // every valid value will be accepted for `group`
        // The capacity for `*groups` is passed in as `*ngroups` which is the buffer max length/capacity (as we initialize with 0)
        // Following reads from `*groups`/`buff` will only happen after `buff.truncate(*ngroups)`
        #[cfg(target_os = "macos")]
        let res = unsafe {
            nix::libc::getgrouplist(
                name.as_ptr(),
                gid.as_raw() as i32,
                buff.as_mut_ptr(),
                &mut count,
            )
        };

        #[cfg(not(target_os = "macos"))]
        let res = unsafe {
            nix::libc::getgrouplist(name.as_ptr(), gid.as_raw(), buff.as_mut_ptr(), &mut count)
        };

        if res < 0 {
            None
        } else {
            buff.truncate(count as usize);
            buff.sort_unstable();
            buff.dedup();
            // allow trivial cast: on macos i is i32, on linux it's already gid_t
            #[allow(trivial_numeric_casts)]
            Some(
                buff.into_iter()
                    .map(|id| Gid::from_raw(id as gid_t))
                    .filter_map(get_group_by_gid)
                    .map(|group| group.gid)
                    .collect(),
            )
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
