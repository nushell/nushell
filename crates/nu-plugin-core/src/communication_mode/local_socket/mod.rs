use std::ffi::{OsStr, OsString};

#[cfg(test)]
pub(crate) mod tests;

/// Generate a name to be used for a local socket specific to this `nu` process, described by the
/// given `unique_id`, which should be unique to the purpose of the socket.
///
/// On Unix, this is a path, which should generally be 100 characters or less for compatibility. On
/// Windows, this is a name within the `\\.\pipe` namespace.
#[cfg(unix)]
pub fn make_local_socket_name(unique_id: &str) -> OsString {
    // Prefer to put it in XDG_RUNTIME_DIR if set, since that's user-local
    let mut base = if let Some(runtime_dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        std::path::PathBuf::from(runtime_dir)
    } else {
        // Use std::env::temp_dir() for portability, especially since on Android this is probably
        // not `/tmp`
        std::env::temp_dir()
    };
    let socket_name = format!("nu.{}.{}.sock", std::process::id(), unique_id);
    base.push(socket_name);
    base.into()
}

/// Interpret a local socket name for use with `interprocess`.
#[cfg(unix)]
pub fn interpret_local_socket_name(
    name: &OsStr,
) -> Result<interprocess::local_socket::Name<'_>, std::io::Error> {
    use interprocess::local_socket::{GenericFilePath, ToFsName};

    name.to_fs_name::<GenericFilePath>()
}

/// Generate a name to be used for a local socket specific to this `nu` process, described by the
/// given `unique_id`, which should be unique to the purpose of the socket.
///
/// On Unix, this is a path, which should generally be 100 characters or less for compatibility. On
/// Windows, this is a name within the `\\.\pipe` namespace.
#[cfg(windows)]
pub fn make_local_socket_name(unique_id: &str) -> OsString {
    format!("nu.{}.{}", std::process::id(), unique_id).into()
}

/// Interpret a local socket name for use with `interprocess`.
#[cfg(windows)]
pub fn interpret_local_socket_name(
    name: &OsStr,
) -> Result<interprocess::local_socket::Name<'_>, std::io::Error> {
    use interprocess::local_socket::{GenericNamespaced, ToNsName};

    name.to_ns_name::<GenericNamespaced>()
}

/// Determine if the error is just due to the listener not being ready yet in asynchronous mode
#[cfg(not(windows))]
pub fn is_would_block_err(err: &std::io::Error) -> bool {
    err.kind() == std::io::ErrorKind::WouldBlock
}

/// Determine if the error is just due to the listener not being ready yet in asynchronous mode
#[cfg(windows)]
pub fn is_would_block_err(err: &std::io::Error) -> bool {
    err.kind() == std::io::ErrorKind::WouldBlock
        || err.raw_os_error().is_some_and(|e| {
            // Windows returns this error when trying to accept a pipe in non-blocking mode
            e as i64 == windows::Win32::Foundation::ERROR_PIPE_LISTENING.0 as i64
        })
}
