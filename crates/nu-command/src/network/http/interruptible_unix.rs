//! Interruptible Unix domain socket connector for ureq HTTP client.
//!
//! This module provides a Unix socket transport implementation that can be interrupted
//! when the user presses Ctrl+C by storing a cloned socket handle.

use std::fmt;
use std::io::{Read, Write};
use std::path::PathBuf;

#[cfg(unix)]
use std::os::unix::net::UnixStream;
#[cfg(windows)]
use win_uds::net::UnixStream;

use ureq::Error;
use ureq::unversioned::transport::{
    Buffers, ConnectionDetails, Connector, LazyBuffers, NextTimeout, Transport,
};

use super::interruptible_stream::ActiveConnections;

/// Connector for interruptible Unix domain sockets.
///
/// This connector stores cloned socket handles in ActiveConnections,
/// allowing them to be shutdown when Ctrl+C is pressed.
#[derive(Debug)]
pub struct InterruptibleUnixSocketConnector {
    socket_path: PathBuf,
    active_connections: ActiveConnections,
}

impl InterruptibleUnixSocketConnector {
    /// Create a new interruptible Unix socket connector.
    pub fn new(socket_path: PathBuf, active_connections: ActiveConnections) -> Self {
        Self {
            socket_path,
            active_connections,
        }
    }
}

impl<In: Transport> Connector<In> for InterruptibleUnixSocketConnector {
    type Out = InterruptibleUnixSocketTransport;

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

        // Clone the stream and store for interruption
        if let Ok(clone) = stream.try_clone() {
            self.active_connections.add(clone);
        }

        let buffers = LazyBuffers::new(
            details.config.input_buffer_size(),
            details.config.output_buffer_size(),
        );

        Ok(Some(InterruptibleUnixSocketTransport::new(stream, buffers)))
    }
}

/// Transport implementation for interruptible Unix domain sockets.
pub struct InterruptibleUnixSocketTransport {
    stream: UnixStream,
    buffers: LazyBuffers,
}

impl InterruptibleUnixSocketTransport {
    /// Create a new Unix socket transport.
    pub fn new(stream: UnixStream, buffers: LazyBuffers) -> Self {
        Self { stream, buffers }
    }
}

impl Transport for InterruptibleUnixSocketTransport {
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

impl fmt::Debug for InterruptibleUnixSocketTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InterruptibleUnixSocketTransport")
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
        let active_connections = ActiveConnections::new();
        let connector = InterruptibleUnixSocketConnector::new(path.clone(), active_connections);
        // Verify via Debug implementation since socket_path is private
        let debug_str = format!("{connector:?}");
        assert!(debug_str.contains("InterruptibleUnixSocketConnector"));
        assert!(debug_str.contains("/tmp/test.sock"));
    }

    #[test]
    fn test_connector_stores_path() {
        let active_connections = ActiveConnections::new();
        let connector = InterruptibleUnixSocketConnector::new("/var/run/docker.sock".into(), active_connections);
        let debug_str = format!("{connector:?}");
        assert!(debug_str.contains("/var/run/docker.sock"));
    }
}
