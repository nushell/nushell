//! Unix domain socket connector for ureq HTTP client.
//!
//! This module provides a custom transport implementation for connecting to HTTP servers
//! over Unix domain sockets, commonly used for Docker API, systemd, and other local services.

use std::fmt;
use std::io::{Read, Write};
use std::path::PathBuf;

#[cfg(unix)]
use std::os::unix::net::UnixStream;
#[cfg(windows)]
use uds_windows::UnixStream;

use ureq::Error;
use ureq::unversioned::transport::{
    Buffers, ConnectionDetails, Connector, LazyBuffers, NextTimeout, Transport,
};

/// Connector for Unix domain sockets.
///
/// This connector ignores the resolved addresses from DNS and instead connects
/// to the specified Unix socket path.
#[derive(Debug)]
pub struct UnixSocketConnector {
    socket_path: PathBuf,
}

impl UnixSocketConnector {
    /// Create a new Unix socket connector with the given socket path.
    pub fn new(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }
}

impl<In: Transport> Connector<In> for UnixSocketConnector {
    type Out = UnixSocketTransport;

    fn connect(
        &self,
        details: &ConnectionDetails,
        _chained: Option<In>,
    ) -> Result<Option<Self::Out>, Error> {
        // Connect to the Unix socket, ignoring the URI's host/port
        let stream = UnixStream::connect(&self.socket_path).map_err(|e| {
            Error::Io(std::io::Error::new(
                e.kind(),
                format!(
                    "Failed to connect to Unix socket {:?}: {}",
                    self.socket_path, e
                ),
            ))
        })?;

        let buffers = LazyBuffers::new(
            details.config.input_buffer_size(),
            details.config.output_buffer_size(),
        );

        Ok(Some(UnixSocketTransport::new(stream, buffers)))
    }
}

/// Transport implementation for Unix domain sockets.
///
/// Wraps a `UnixStream` and implements the `Transport` trait required by ureq.
pub struct UnixSocketTransport {
    stream: UnixStream,
    buffers: LazyBuffers,
}

impl UnixSocketTransport {
    /// Create a new Unix socket transport.
    pub fn new(stream: UnixStream, buffers: LazyBuffers) -> Self {
        Self { stream, buffers }
    }
}

impl Transport for UnixSocketTransport {
    fn buffers(&mut self) -> &mut dyn Buffers {
        &mut self.buffers
    }

    fn transmit_output(&mut self, amount: usize, _timeout: NextTimeout) -> Result<(), Error> {
        let output = &self.buffers.output()[..amount];
        self.stream.write_all(output).map_err(Error::Io)?;
        Ok(())
    }

    fn await_input(&mut self, _timeout: NextTimeout) -> Result<bool, Error> {
        let input = self.buffers.input_append_buf();
        let amount = self.stream.read(input).map_err(Error::Io)?;
        self.buffers.input_appended(amount);
        Ok(amount > 0)
    }

    fn is_open(&mut self) -> bool {
        // Unix sockets don't have a reliable way to check connection status
        // without using unstable features, so we assume the connection is open.
        // The connection will be detected as closed when we try to read/write.
        true
    }
}

impl fmt::Debug for UnixSocketTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnixSocketTransport")
            .field("peer_addr", &self.stream.peer_addr().ok())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connector_creation() {
        let path = PathBuf::from("/tmp/test.sock");
        let connector = UnixSocketConnector::new(path.clone());
        // Verify via Debug implementation since socket_path is private
        let debug_str = format!("{connector:?}");
        assert!(debug_str.contains("UnixSocketConnector"));
        assert!(debug_str.contains("/tmp/test.sock"));
    }

    #[test]
    fn test_connector_stores_path() {
        let connector = UnixSocketConnector::new("/var/run/docker.sock".into());
        let debug_str = format!("{connector:?}");
        assert!(debug_str.contains("/var/run/docker.sock"));
    }
}
