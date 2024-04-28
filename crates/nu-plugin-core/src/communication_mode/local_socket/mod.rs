use std::ffi::OsString;

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

/// Generate a name to be used for a local socket specific to this `nu` process, described by the
/// given `unique_id`, which should be unique to the purpose of the socket.
///
/// On Unix, this is a path, which should generally be 100 characters or less for compatibility. On
/// Windows, this is a name within the `\\.\pipe` namespace.
#[cfg(windows)]
pub fn make_local_socket_name(unique_id: &str) -> OsString {
    format!("nu.{}.{}", std::process::id(), unique_id).into()
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

/// Wraps the `interprocess` local socket stream for greater compatibility
#[derive(Debug)]
pub struct LocalSocketStream(pub interprocess::local_socket::LocalSocketStream);

impl From<interprocess::local_socket::LocalSocketStream> for LocalSocketStream {
    fn from(value: interprocess::local_socket::LocalSocketStream) -> Self {
        LocalSocketStream(value)
    }
}

impl std::io::Read for LocalSocketStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}

impl std::io::Write for LocalSocketStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        // We don't actually flush the underlying socket on Windows. The flush operation on a
        // Windows named pipe actually synchronizes with read on the other side, and won't finish
        // until the other side is empty. This isn't how most of our other I/O methods work, so we
        // just won't do it. The BufWriter above this will have still made a write call with the
        // contents of the buffer, which should be good enough.
        if cfg!(not(windows)) {
            self.0.flush()?;
        }
        Ok(())
    }
}
