//! Interruptible Unix domain socket connector for ureq HTTP client.
//!
//! This module provides a Unix socket transport implementation that can be interrupted
//! when the user presses Ctrl+C via a registered signal handler.
//!
//! See [`super::interruptible_tcp`] for a detailed explanation of the interrupt strategy.

use std::fmt;
use std::io::{Read, Write};
use std::net::Shutdown;
use std::path::PathBuf;
use std::sync::Arc;

#[cfg(unix)]
use std::os::unix::net::UnixStream;
#[cfg(windows)]
use win_uds::net::UnixStream;

use nu_protocol::HandlerGuard;
use ureq::Error;
use ureq::unversioned::transport::{
    Buffers, ConnectionDetails, Connector, LazyBuffers, NextTimeout, Transport,
};

/// Callback invoked when a connection is established.
/// Takes a cloned socket and returns a guard that keeps the interrupt handler registered.
pub type OnConnectUnix = Arc<dyn Fn(UnixStream) -> Option<HandlerGuard> + Send + Sync>;

/// Connector for interruptible Unix domain sockets.
///
/// When a connection is established, calls the `on_connect` callback with a cloned
/// socket handle. The callback registers a signal handler and returns a guard.
pub struct InterruptibleUnixSocketConnector {
    socket_path: PathBuf,
    on_connect: Option<OnConnectUnix>,
}

impl InterruptibleUnixSocketConnector {
    /// Create a new interruptible Unix socket connector.
    pub fn new(socket_path: PathBuf, on_connect: Option<OnConnectUnix>) -> Self {
        Self {
            socket_path,
            on_connect,
        }
    }
}

impl fmt::Debug for InterruptibleUnixSocketConnector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InterruptibleUnixSocketConnector")
            .field("socket_path", &self.socket_path)
            .field("on_connect", &self.on_connect.as_ref().map(|_| "..."))
            .finish()
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

        // Register interrupt handler if callback provided
        let guard = self
            .on_connect
            .as_ref()
            .and_then(|f| stream.try_clone().ok().and_then(|s| f(s)));

        let buffers = LazyBuffers::new(
            details.config.input_buffer_size(),
            details.config.output_buffer_size(),
        );

        Ok(Some(InterruptibleUnixSocketTransport::new(
            stream, buffers, guard,
        )))
    }
}

/// Transport implementation for interruptible Unix domain sockets.
///
/// Holds a guard that keeps the interrupt handler registered while the transport is alive.
pub struct InterruptibleUnixSocketTransport {
    stream: UnixStream,
    buffers: LazyBuffers,
    _guard: Option<HandlerGuard>,
}

impl InterruptibleUnixSocketTransport {
    /// Create a new Unix socket transport.
    pub fn new(stream: UnixStream, buffers: LazyBuffers, guard: Option<HandlerGuard>) -> Self {
        Self {
            stream,
            buffers,
            _guard: guard,
        }
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

/// Create an `OnConnectUnix` callback that registers a signal handler to shutdown the socket.
pub fn make_on_connect_unix(handlers: &nu_protocol::Handlers) -> OnConnectUnix {
    let handlers = handlers.clone();
    Arc::new(move |socket: UnixStream| {
        handlers
            .register(Box::new(move |action| {
                if matches!(action, nu_protocol::SignalAction::Interrupt) {
                    let _ = socket.shutdown(Shutdown::Both);
                }
            }))
            .ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connector_creation() {
        let path = PathBuf::from("/tmp/test.sock");
        let connector = InterruptibleUnixSocketConnector::new(path.clone(), None);
        let debug_str = format!("{connector:?}");
        assert!(debug_str.contains("InterruptibleUnixSocketConnector"));
        assert!(debug_str.contains("/tmp/test.sock"));
    }

    #[test]
    fn test_connector_stores_path() {
        let connector = InterruptibleUnixSocketConnector::new("/var/run/docker.sock".into(), None);
        let debug_str = format!("{connector:?}");
        assert!(debug_str.contains("/var/run/docker.sock"));
    }
}
