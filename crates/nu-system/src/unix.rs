/// Returns the umask for the current process.
///
/// This reads the umask by setting it to 0 and back. A lock keeps two callers of this
/// function from interleaving, but another thread that changes the umask at the same time
/// can still race it.
#[deprecated(note = "use `nu_command::get_umask`, which also coordinates with the uutils commands")]
pub fn get_umask() -> u32 {
    use nix::sys::stat::{Mode, umask};

    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = LOCK.lock().unwrap_or_else(|poisoned| {
        log::warn!("umask lock poisoned. Recovering.");
        poisoned.into_inner()
    });
    let mask = umask(Mode::empty());
    umask(mask);
    u32::from(mask.bits())
}

/// Resolves user ids to account names for a process listing, looking each id up only once.
///
/// Many processes share a handful of users, and each lookup can go through the system's user
/// database (e.g. Open Directory on macOS), so caching keeps `ps` fast.
#[derive(Default)]
pub(crate) struct UserNames(std::collections::HashMap<u32, Option<String>>);

impl UserNames {
    /// Returns the account name for `uid`, or `None` if no account has that id.
    pub(crate) fn get(&mut self, uid: u32) -> Option<String> {
        self.0
            .entry(uid)
            .or_insert_with(|| {
                nu_utils::filesystem::users::get_user_by_uid(uid.into()).map(|user| user.name)
            })
            .clone()
    }
}

/// Splits NUL-separated strings, such as a process's environment, skipping empty entries.
#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
pub(crate) fn split_nul(bytes: &[u8]) -> Vec<String> {
    bytes
        .split(|b| *b == 0)
        .filter(|entry| !entry.is_empty())
        .map(|entry| String::from_utf8_lossy(entry).into_owned())
        .collect()
}

/// Reads a NUL-terminated string from the start of `bytes`, or `None` if it is empty.
#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
pub(crate) fn c_string(bytes: &[u8]) -> Option<String> {
    let string = std::ffi::CStr::from_bytes_until_nul(bytes)
        .ok()?
        .to_string_lossy();
    (!string.is_empty()).then(|| string.into_owned())
}

#[cfg(test)]
mod tests {
    #[test]
    #[allow(deprecated)]
    fn get_umask_restores_the_umask() {
        // A second read would return 0 if the first one didn't put the umask back.
        let first = super::get_umask();
        assert_eq!(first, super::get_umask());
    }
}
