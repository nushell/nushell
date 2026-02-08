//! Interruptible TCP connector for ureq HTTP client.
//!
//! # Interrupt Strategy
//!
//! Nushell uses sync I/O for simplicity. Downstream response handling consumes data
//! through the [`Read`] trait, which only offers blocking `read()` with no timeout,
//! so we cannot poll for interrupts.
//!
//! Solution: socket shutdown from the signal handler.
//!
//! 1. On connect, clone the socket via [`TcpStream::try_clone`] (second handle, same OS socket).
//! 2. Pass the clone to an [`OnConnect`] callback that registers it with Nushell's
//!    signal handlers, returning a [`HandlerGuard`] stored in the transport.
//! 3. On Ctrl+C, the handler calls `shutdown(Shutdown::Both)` on the cloned handle.
//! 4. Any blocked `read()`/`write()` on the original handle returns an error immediately.
//!
//! No polling, no extra threads. The read unblocks because the socket has been closed.

use std::fmt;
use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::sync::Arc;
use std::time::Duration;

use nu_protocol::HandlerGuard;
use ureq::Error;
use ureq::unversioned::transport::{
    Buffers, ConnectionDetails, Connector, LazyBuffers, NextTimeout, Transport,
};

/// Callback invoked when a connection is established.
/// Takes a cloned socket and returns a guard that keeps the interrupt handler registered.
pub type OnConnect = Arc<dyn Fn(TcpStream) -> Option<HandlerGuard> + Send + Sync>;

/// Connector for interruptible TCP sockets.
///
/// When a connection is established, calls the `on_connect` callback with a cloned
/// socket handle. The callback registers a signal handler and returns a guard.
pub struct InterruptibleTcpConnector {
    on_connect: Option<OnConnect>,
}

impl InterruptibleTcpConnector {
    /// Create a new interruptible TCP connector.
    pub fn new(on_connect: Option<OnConnect>) -> Self {
        Self { on_connect }
    }
}

impl fmt::Debug for InterruptibleTcpConnector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InterruptibleTcpConnector")
            .field("on_connect", &self.on_connect.as_ref().map(|_| "..."))
            .finish()
    }
}

impl<In: Transport> Connector<In> for InterruptibleTcpConnector {
    type Out = InterruptibleTcpTransport;

    fn connect(
        &self,
        details: &ConnectionDetails,
        _chained: Option<In>,
    ) -> Result<Option<Self::Out>, Error> {
        let stream = try_connect(details)?;

        // Register interrupt handler if callback provided.
        // If try_clone() fails, we proceed without interrupt handling - the request
        // will still work, just won't respond to Ctrl+C until data arrives.
        let guard = self
            .on_connect
            .as_ref()
            .and_then(|f| stream.try_clone().ok().and_then(|s| f(s)));

        let buffers = LazyBuffers::new(
            details.config.input_buffer_size(),
            details.config.output_buffer_size(),
        );

        Ok(Some(InterruptibleTcpTransport::new(stream, buffers, guard)))
    }
}

fn try_connect(details: &ConnectionDetails) -> Result<TcpStream, Error> {
    let timeout = details.timeout;
    let mut last_error = None;

    // Iterate over the resolved addresses
    for addr in details.addrs.iter() {
        match try_connect_single(*addr, timeout) {
            Ok(stream) => return Ok(stream),
            Err(Error::Io(e)) if e.kind() == std::io::ErrorKind::ConnectionRefused => {
                last_error = Some(Error::Io(e));
                continue;
            }
            Err(e) => return Err(e),
        }
    }

    Err(last_error.unwrap_or(Error::ConnectionFailed))
}

fn try_connect_single(addr: SocketAddr, timeout: NextTimeout) -> Result<TcpStream, Error> {
    let maybe_stream = if let Some(t) = timeout.not_zero() {
        TcpStream::connect_timeout(&addr, *t)
    } else {
        TcpStream::connect(addr)
    };

    // Match ureq's normalize_would_block behavior: WouldBlock -> TimedOut
    let stream = match maybe_stream {
        Ok(s) => s,
        Err(e)
            if e.kind() == std::io::ErrorKind::TimedOut
                || e.kind() == std::io::ErrorKind::WouldBlock =>
        {
            return Err(Error::Timeout(timeout.reason));
        }
        Err(e) => return Err(Error::Io(e)),
    };

    // Set TCP_NODELAY for better latency
    let _ = stream.set_nodelay(true);

    Ok(stream)
}

/// Transport implementation for interruptible TCP sockets.
///
/// Holds a guard that keeps the interrupt handler registered while the transport is alive.
pub struct InterruptibleTcpTransport {
    stream: TcpStream,
    buffers: LazyBuffers,
    timeout_write: Option<Duration>,
    timeout_read: Option<Duration>,
    _guard: Option<HandlerGuard>,
}

impl InterruptibleTcpTransport {
    /// Create a new TCP transport.
    pub fn new(stream: TcpStream, buffers: LazyBuffers, guard: Option<HandlerGuard>) -> Self {
        Self {
            stream,
            buffers,
            timeout_read: None,
            timeout_write: None,
            _guard: guard,
        }
    }
}

impl Transport for InterruptibleTcpTransport {
    fn buffers(&mut self) -> &mut dyn Buffers {
        &mut self.buffers
    }

    fn transmit_output(&mut self, amount: usize, timeout: NextTimeout) -> Result<(), Error> {
        maybe_update_timeout(
            timeout,
            &mut self.timeout_write,
            &self.stream,
            TcpStream::set_write_timeout,
        )?;

        let output = &self.buffers.output()[..amount];
        match self.stream.write_all(output) {
            Ok(()) => Ok(()),
            // Match ureq's normalize_would_block behavior: WouldBlock -> TimedOut
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
        maybe_update_timeout(
            timeout,
            &mut self.timeout_read,
            &self.stream,
            TcpStream::set_read_timeout,
        )?;

        let input = self.buffers.input_append_buf();
        let amount = match self.stream.read(input) {
            Ok(n) => n,
            // Match ureq's normalize_would_block behavior: WouldBlock -> TimedOut
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
        // Try a non-blocking read to check if connection is still alive
        self.stream.set_nonblocking(true).ok();
        let mut buf = [0u8; 1];
        let result = match self.stream.peek(&mut buf) {
            Ok(_) => true,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => true,
            Err(_) => false,
        };
        self.stream.set_nonblocking(false).ok();
        result
    }
}

impl fmt::Debug for InterruptibleTcpTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InterruptibleTcpTransport")
            .field("peer_addr", &self.stream.peer_addr().ok())
            .finish()
    }
}

fn maybe_update_timeout(
    timeout: NextTimeout,
    previous: &mut Option<Duration>,
    stream: &TcpStream,
    f: impl Fn(&TcpStream, Option<Duration>) -> std::io::Result<()>,
) -> Result<(), Error> {
    let maybe_timeout = timeout.not_zero().map(|t| *t);

    if maybe_timeout != *previous {
        f(stream, maybe_timeout).map_err(Error::Io)?;
        *previous = maybe_timeout;
    }

    Ok(())
}

/// Create an `OnConnect` callback that registers a signal handler to shutdown the socket.
pub fn make_on_connect(handlers: &nu_protocol::Handlers) -> OnConnect {
    let handlers = handlers.clone();
    Arc::new(move |socket: TcpStream| {
        handlers
            .register(Box::new(move |action| {
                if matches!(action, nu_protocol::SignalAction::Interrupt) {
                    let _ = socket.shutdown(Shutdown::Both);
                }
            }))
            .ok()
    })
}
