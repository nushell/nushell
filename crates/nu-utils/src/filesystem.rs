#[cfg(any(windows, unix))]
use std::path::Path;
#[cfg(unix)]
use {
    nix::{
        sys::stat::{mode_t, Mode},
        unistd::{Gid, Uid},
    },
    std::os::unix::fs::MetadataExt,
};

// The result of checking whether we have permission to cd to a directory
#[derive(Debug)]
pub enum PermissionResult<'a> {
    PermissionOk,
    PermissionDenied(&'a str),
}

// TODO: Maybe we should use file_attributes() from https://doc.rust-lang.org/std/os/windows/fs/trait.MetadataExt.html
// More on that here: https://learn.microsoft.com/en-us/windows/win32/fileio/file-attribute-constants
#[cfg(windows)]
pub fn have_permission(dir: impl AsRef<Path>) -> PermissionResult<'static> {
    match dir.as_ref().read_dir() {
        Err(e) => {
            if matches!(e.kind(), std::io::ErrorKind::PermissionDenied) {
                PermissionResult::PermissionDenied("Folder is unable to be read")
            } else {
                PermissionResult::PermissionOk
            }
        }
        Ok(_) => PermissionResult::PermissionOk,
    }
}

#[cfg(unix)]
pub fn have_permission(dir: impl AsRef<Path>) -> PermissionResult<'static> {
    match dir.as_ref().metadata() {
        Ok(metadata) => {
            let mode = Mode::from_bits_truncate(metadata.mode() as mode_t);
            let current_user_uid = users::get_current_uid();
            if current_user_uid.is_root() {
                return PermissionResult::PermissionOk;
            }
            let current_user_gid = users::get_current_gid();
            let owner_user = Uid::from_raw(metadata.uid());
            let owner_group = Gid::from_raw(metadata.gid());
            match (
                current_user_uid == owner_user,
                current_user_gid == owner_group,
            ) {
                (true, _) => {
                    if mode.contains(Mode::S_IXUSR) {
                        PermissionResult::PermissionOk
                    } else {
                        PermissionResult::PermissionDenied(
                            "You are the owner but do not have execute permission",
                        )
                    }
                }
                (false, true) => {
                    if mode.contains(Mode::S_IXGRP) {
                        PermissionResult::PermissionOk
                    } else {
                        PermissionResult::PermissionDenied(
                            "You are in the group but do not have execute permission",
                        )
                    }
                }
                (false, false) => {
                    if mode.contains(Mode::S_IXOTH)
                        || (mode.contains(Mode::S_IXGRP)
                            && any_group(current_user_gid, owner_group))
                    {
                        PermissionResult::PermissionOk
                    } else {
                        PermissionResult::PermissionDenied(
                            "You are neither the owner, in the group, nor the super user and do not have permission",
                        )
                    }
                }
            }
        }
        Err(_) => PermissionResult::PermissionDenied("Could not retrieve file metadata"),
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "android"))]
fn any_group(_current_user_gid: Gid, owner_group: Gid) -> bool {
    users::current_user_groups()
        .unwrap_or_default()
        .contains(&owner_group)
}

#[cfg(all(
    unix,
    not(any(target_os = "linux", target_os = "freebsd", target_os = "android"))
))]
fn any_group(current_user_gid: Gid, owner_group: Gid) -> bool {
    users::get_current_username()
        .and_then(|name| users::get_user_groups(&name, current_user_gid))
        .unwrap_or_default()
        .contains(&owner_group)
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
