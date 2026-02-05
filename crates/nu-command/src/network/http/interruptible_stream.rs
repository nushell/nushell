//! Interruptible stream infrastructure for HTTP connections.
//!
//! This module provides the ability to interrupt blocked I/O operations on HTTP connections
//! when the user presses Ctrl+C. It works by storing cloned stream handles that can be
//! shutdown from another thread, causing blocked reads to return immediately.

use std::io::{self, Read};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

use nu_protocol::HandlerGuard;

#[cfg(unix)]
use std::os::unix::net::UnixStream;
#[cfg(windows)]
use win_uds::net::UnixStream;

/// Trait for stream handles that can be shutdown to interrupt blocked I/O.
pub trait ShutdownHandle: Send + Sync + 'static {
    /// Shutdown the stream, causing any blocked reads/writes to return.
    fn shutdown(&self) -> io::Result<()>;
}

impl ShutdownHandle for TcpStream {
    fn shutdown(&self) -> io::Result<()> {
        TcpStream::shutdown(self, std::net::Shutdown::Both)
    }
}

impl ShutdownHandle for UnixStream {
    fn shutdown(&self) -> io::Result<()> {
        UnixStream::shutdown(self, std::net::Shutdown::Both)
    }
}

/// Storage for active connections that can be interrupted.
///
/// When Ctrl+C is pressed, all stored connections will be shutdown,
/// causing any blocked I/O operations to return immediately.
#[derive(Default, Clone)]
pub struct ActiveConnections {
    connections: Arc<Mutex<Vec<Box<dyn ShutdownHandle>>>>,
}

impl ActiveConnections {
    /// Create a new empty connection store.
    pub fn new() -> Self {
        Self {
            connections: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Add a connection to be tracked.
    pub fn add<S: ShutdownHandle>(&self, stream: S) {
        if let Ok(mut conns) = self.connections.lock() {
            conns.push(Box::new(stream));
        }
    }

    /// Shutdown all active connections.
    ///
    /// This will cause any blocked reads/writes on these connections to return.
    /// Errors during shutdown are ignored since we're in an interrupt context.
    pub fn shutdown_all(&self) {
        if let Ok(mut conns) = self.connections.lock() {
            for conn in conns.drain(..) {
                let _ = conn.shutdown();
            }
        }
    }

}

impl std::fmt::Debug for ActiveConnections {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let count = self
            .connections
            .lock()
            .map(|c| c.len())
            .unwrap_or(0);
        f.debug_struct("ActiveConnections")
            .field("count", &count)
            .finish()
    }
}

/// A reader wrapper that holds a HandlerGuard.
///
/// The guard is kept alive as long as this reader exists, ensuring the
/// interrupt handler stays registered while the response body is being read.
/// When the reader is dropped (stream consumed or interrupted), the guard
/// is dropped and the handler is unregistered.
pub struct GuardedReader<R> {
    inner: R,
    _guard: HandlerGuard,
}

impl<R> GuardedReader<R> {
    /// Wrap a reader with a handler guard.
    pub fn new(inner: R, guard: HandlerGuard) -> Self {
        Self {
            inner,
            _guard: guard,
        }
    }
}

impl<R: Read> Read for GuardedReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}
