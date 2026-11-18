//! Local sockets that a `nu` process opens for a process it starts: a Unix domain socket in an
//! owner-only directory, or a named pipe on Windows.
//!
//! Plugins use them to talk to the engine without tying up stdio, and a `nu` started with
//! `--structured-io` uses one to pass values to and from the `nu` that started it.

use interprocess::local_socket::{Listener, ListenerOptions, Name};
#[cfg(unix)]
use std::path::Path;
use std::{
    ffi::{OsStr, OsString},
    io,
    path::PathBuf,
};

/// Generate a name to be used for a local socket specific to this `nu` process, described by the
/// given `unique_id`, which should be unique to the purpose of the socket.
///
/// On Unix, this is a path, which should generally be 100 characters or less for compatibility. On
/// Windows, this is a name within the `\\.\pipe` namespace.
#[cfg(unix)]
pub fn make_local_socket_name(unique_id: &str) -> OsString {
    // Prefer to put it in XDG_RUNTIME_DIR if set, since that's user-local
    let mut base = if let Some(runtime_dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir)
    } else {
        // Use std::env::temp_dir() for portability, especially since on Android this is probably
        // not `/tmp`
        std::env::temp_dir()
    };
    let socket_dir = format!("nu.{}.{}", std::process::id(), unique_id);
    base.push(socket_dir);
    base.push("plugin.sock");
    base.into()
}

/// Interpret a local socket name for use with `interprocess`.
#[cfg(unix)]
pub fn interpret_local_socket_name(name: &OsStr) -> Result<Name<'_>, io::Error> {
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
pub fn interpret_local_socket_name(name: &OsStr) -> Result<Name<'_>, io::Error> {
    use interprocess::local_socket::{GenericNamespaced, ToNsName};

    name.to_ns_name::<GenericNamespaced>()
}

/// Determine if the error is just due to the listener not being ready yet in asynchronous mode
#[cfg(not(windows))]
pub fn is_would_block_err(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::WouldBlock
}

/// Determine if the error is just due to the listener not being ready yet in asynchronous mode
#[cfg(windows)]
pub fn is_would_block_err(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::WouldBlock
        || err.raw_os_error().is_some_and(|e| {
            // Windows returns this error when trying to accept a pipe in non-blocking mode
            e as i64 == windows_sys::Win32::Foundation::ERROR_PIPE_LISTENING as i64
        })
}

/// Creates a listener on the local socket `name`. On Unix, the socket's directory is created
/// first, readable by this user only, and returned so the caller can remove it with
/// [`cleanup_local_socket_dir`] once the socket isn't needed anymore. It's removed here if the
/// listener can't be created.
pub fn bind_local_socket(name: &OsStr) -> io::Result<(Listener, Option<PathBuf>)> {
    #[cfg(unix)]
    let socket_dir = prepare_local_socket_path(name)?;
    #[cfg(not(unix))]
    let socket_dir = None;

    match interpret_local_socket_name(name)
        .and_then(|name| ListenerOptions::new().name(name).create_sync())
    {
        Ok(listener) => Ok((listener, socket_dir)),
        Err(err) => {
            cleanup_local_socket_dir(socket_dir.as_ref());
            Err(err)
        }
    }
}

/// Creates the directory for the socket at `name`, readable by this user only. Fails if it already
/// exists, so a directory someone else made can't be used to listen in.
#[cfg(unix)]
pub fn prepare_local_socket_path(name: &OsStr) -> Result<Option<PathBuf>, io::Error> {
    use std::os::unix::fs::DirBuilderExt;

    let path = Path::new(name);
    let Some(parent) = path.parent() else {
        return Ok(None);
    };

    let mut builder = std::fs::DirBuilder::new();
    builder.mode(0o700).create(parent)?;
    Ok(Some(parent.to_path_buf()))
}

/// Removes a socket directory made by [`prepare_local_socket_path`] or [`bind_local_socket`].
pub fn cleanup_local_socket_dir(socket_dir: Option<&PathBuf>) {
    if let Some(socket_dir) = socket_dir {
        let _ = std::fs::remove_dir_all(socket_dir);
    }
}

#[cfg(test)]
mod tests {
    use super::make_local_socket_name;

    #[test]
    fn local_socket_path_contains_pid() {
        let name = make_local_socket_name("test-string")
            .to_string_lossy()
            .into_owned();
        assert!(name.contains(&std::process::id().to_string()));
    }

    #[test]
    fn local_socket_path_contains_provided_name() {
        let name = make_local_socket_name("test-string")
            .to_string_lossy()
            .into_owned();
        assert!(name.contains("test-string"));
    }
}
