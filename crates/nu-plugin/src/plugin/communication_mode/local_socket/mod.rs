use std::path::PathBuf;

#[cfg(test)]
pub(crate) mod tests;

/// Generate a path to be used for a local socket specific to this `nu` process, described by the
/// given `unique_id`, which should be unique to the purpose of the socket.
///
/// Note that the length of a socket path is limited on many unix platforms.
#[cfg(unix)]
pub fn make_local_socket_path(unique_id: &str) -> PathBuf {
    // Prefer to put it in XDG_RUNTIME_DIR if set, since that's user-local
    let mut base = if let Some(runtime_dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir)
    } else {
        // Use std::env::temp_dir() for portability, especially since on Android this is probably
        // not `/tmp`
        std::env::temp_dir()
    };
    let socket_name = format!("nu.{}.{}.sock", std::process::id(), unique_id);
    base.push(socket_name);
    base
}

/// Generate a path to be used for a local socket specific to this `nu` process, described by the
/// given `unique_id`, which should be unique to the purpose of the socket.
#[cfg(windows)]
pub fn make_local_socket_path(unique_id: &str) -> PathBuf {
    // Windows uses the "\\.\pipe\" filesystem
    let mut base = PathBuf::from(r"\\.\pipe");
    let socket_name = format!("nu.{}.{}", std::process::id(), unique_id);
    base.push(socket_name);
    base
}
