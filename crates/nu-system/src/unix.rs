use log::warn;
use std::sync::Mutex;

/// Returns the umask for the current process
pub fn get_umask() -> u32 {
    // uucore::more::get_umask isn't threadsafe, see:
    // https://github.com/uutils/coreutils/blob/8bb31eeab9abe1c73ac5c03f3930d6e25854e4f2/src/uucore/src/lib/features/mode.rs#L172-L196
    //
    // This lock attempts to fake it.
    static LOCK: Mutex<()> = Mutex::new(());
    let _guard = match LOCK.lock() {
        Ok(g) => g,
        Err(e) => {
            warn!("umask lock poisoned. Recovering.");
            e.into_inner()
        }
    };

    uucore::mode::get_umask()
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
