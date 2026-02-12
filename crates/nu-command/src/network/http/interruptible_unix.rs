//! Interruptible Unix domain socket connector for ureq HTTP client.
//!
//! This module provides a Unix socket transport implementation that can be interrupted
//! when the user presses Ctrl+C via a registered signal handler.
//!
//! See [`super::interruptible_tcp`] for a detailed explanation of the interrupt strategy.

use std::fmt;
use std::io::{Read, Write};
use std::mem::ManuallyDrop;
#[cfg(unix)]
use std::net::Shutdown;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::net::UnixStream;
#[cfg(windows)]
use win_uds::net::UnixStream;

use nu_protocol::HandlerGuard;
use ureq::Error;
use ureq::unversioned::transport::{
    Buffers, ConnectionDetails, Connector, LazyBuffers, NextTimeout, Transport,
};

/// Callback invoked when a Unix socket connection is established.
pub type OnConnectUnix =
    Arc<dyn Fn(&UnixStream) -> Option<(HandlerGuard, Arc<AtomicBool>)> + Send + Sync>;

/// Connector for interruptible Unix domain sockets.
pub struct InterruptibleUnixSocketConnector {
    socket_path: PathBuf,
    on_connect: Option<OnConnectUnix>,
}

impl InterruptibleUnixSocketConnector {
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

        let (guard, closed) = self
            .on_connect
            .as_ref()
            .and_then(|f| f(&stream))
            .map(|(g, c)| (Some(g), c))
            .unwrap_or_else(|| (None, Arc::new(AtomicBool::new(false))));

        let buffers = LazyBuffers::new(
            details.config.input_buffer_size(),
            details.config.output_buffer_size(),
        );

        Ok(Some(InterruptibleUnixSocketTransport::new(
            stream, buffers, guard, closed,
        )))
    }
}

/// Transport implementation for interruptible Unix domain sockets.
pub struct InterruptibleUnixSocketTransport {
    stream: ManuallyDrop<UnixStream>,
    buffers: LazyBuffers,
    timeout_write: Option<Duration>,
    timeout_read: Option<Duration>,
    closed: Arc<AtomicBool>,
    _guard: Option<HandlerGuard>,
}

impl InterruptibleUnixSocketTransport {
    pub fn new(
        stream: UnixStream,
        buffers: LazyBuffers,
        guard: Option<HandlerGuard>,
        closed: Arc<AtomicBool>,
    ) -> Self {
        Self {
            stream: ManuallyDrop::new(stream),
            buffers,
            timeout_read: None,
            timeout_write: None,
            closed,
            _guard: guard,
        }
    }
}

impl Drop for InterruptibleUnixSocketTransport {
    fn drop(&mut self) {
        if !self.closed.swap(true, Ordering::AcqRel) {
            // SAFETY: We're in Drop, stream won't be used after this.
            unsafe { ManuallyDrop::drop(&mut self.stream) };
        }
    }
}

impl Transport for InterruptibleUnixSocketTransport {
    fn buffers(&mut self) -> &mut dyn Buffers {
        &mut self.buffers
    }

    fn transmit_output(&mut self, amount: usize, timeout: NextTimeout) -> Result<(), Error> {
        let maybe_timeout = timeout.not_zero().map(|t| *t);
        if maybe_timeout != self.timeout_write {
            self.stream
                .set_write_timeout(maybe_timeout)
                .map_err(Error::Io)?;
            self.timeout_write = maybe_timeout;
        }

        let output = &self.buffers.output()[..amount];
        match self.stream.write_all(output) {
            Ok(()) => Ok(()),
            Err(e)
                if e.kind() == std::io::ErrorKind::TimedOut
                    || e.kind() == std::io::ErrorKind::WouldBlock =>
            {
                Err(Error::Timeout(timeout.reason))
            }
            Err(e) => Err(Error::Io(e)),
        }
    }

    fn await_input(&mut self, timeout: NextTimeout) -> Result<bool, Error> {
        let maybe_timeout = timeout.not_zero().map(|t| *t);
        if maybe_timeout != self.timeout_read {
            self.stream
                .set_read_timeout(maybe_timeout)
                .map_err(Error::Io)?;
            self.timeout_read = maybe_timeout;
        }

        let input = self.buffers.input_append_buf();
        let amount = match self.stream.read(input) {
            Ok(n) => n,
            Err(e)
                if e.kind() == std::io::ErrorKind::TimedOut
                    || e.kind() == std::io::ErrorKind::WouldBlock =>
            {
                return Err(Error::Timeout(timeout.reason));
            }
            Err(e) => return Err(Error::Io(e)),
        };
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

/// Create an `OnConnectUnix` callback that registers a signal handler to interrupt the socket.
///
/// On Unix, the handler calls `shutdown()` via a cloned handle.
/// On Windows, the handler calls `closesocket()` on the original socket handle.
pub fn make_on_connect_unix(handlers: &nu_protocol::Handlers) -> OnConnectUnix {
    let handlers = handlers.clone();
    Arc::new(move |stream: &UnixStream| {
        let closed = Arc::new(AtomicBool::new(false));
        let closed_clone = Arc::clone(&closed);

        #[cfg(unix)]
        let guard = {
            let clone = stream.try_clone().ok()?;
            handlers
                .register(Box::new(move |action| {
                    if matches!(action, nu_protocol::SignalAction::Interrupt) {
                        let _ = clone.shutdown(Shutdown::Both);
                    }
                }))
                .ok()?
        };

        #[cfg(windows)]
        let guard = {
            use std::os::windows::io::AsRawSocket;
            let raw = stream.as_raw_socket() as usize;
            handlers
                .register(Box::new(move |action| {
                    if matches!(action, nu_protocol::SignalAction::Interrupt)
                        && !closed_clone.swap(true, Ordering::AcqRel)
                    {
                        unsafe {
                            windows::Win32::Networking::WinSock::closesocket(
                                windows::Win32::Networking::WinSock::SOCKET(raw),
                            );
                        }
                    }
                }))
                .ok()?
        };

        #[cfg(unix)]
        drop(closed_clone);

        Some((guard, closed))
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

    #[test]
    fn test_interrupt_unblocks_read() {
        use nu_protocol::{Handlers, SignalAction};
        use std::io::Write;
        use std::thread;
        use std::time::{Duration, Instant};

        #[cfg(unix)]
        use std::os::unix::net::UnixListener;
        #[cfg(windows)]
        use win_uds::net::UnixListener;

        let socket_path = std::env::temp_dir().join("nu_test_interrupt.sock");
        let _ = std::fs::remove_file(&socket_path);

        let listener = UnixListener::bind(&socket_path).unwrap();

        let server_thread = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            // Wait longer than our test timeout before sending anything
            thread::sleep(Duration::from_secs(10));
            let _ = stream.write_all(b"delayed response");
        });

        // Set up handlers for interrupt
        let handlers = Handlers::new();
        let on_connect = make_on_connect_unix(&handlers);

        // Connect to the server
        let stream = UnixStream::connect(&socket_path).unwrap();
        // Register the interrupt handler
        let (guard, closed) = on_connect(&stream).unwrap();
        let mut transport = InterruptibleUnixSocketTransport::new(
            stream,
            LazyBuffers::new(8192, 8192),
            Some(guard),
            closed,
        );

        // Start reading in current thread, trigger interrupt from another
        let handlers_clone = handlers.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(100));
            handlers_clone.run(SignalAction::Interrupt);
        });

        let start = Instant::now();
        let mut buf = [0u8; 1024];
        let result = std::io::Read::read(&mut *transport.stream, &mut buf);
        let elapsed = start.elapsed();

        assert!(
            elapsed < Duration::from_secs(2),
            "Read took too long ({elapsed:?}), interrupt may not have worked",
        );

        match result {
            Ok(0) => {}
            Err(_) => {}
            Ok(n) => panic!("Unexpected data received: {n} bytes"),
        }

        let _ = std::fs::remove_file(&socket_path);
        drop(transport);
        drop(server_thread);
    }
}
