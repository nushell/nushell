//! Interruptible TCP connector for ureq HTTP client.
//!
//! # Interrupt Strategy
//!
//! Nushell uses sync I/O for simplicity. Downstream response handling consumes data
//! through the [`Read`] trait, which only offers blocking `read()` with no timeout,
//! so we cannot poll for interrupts.
//!
//! Solution: platform-specific interrupt from the signal handler.
//!
//! ## Unix
//! 1. On connect, clone the socket via [`TcpStream::try_clone`].
//! 2. On Ctrl+C, call `shutdown(Both)` on the clone -- the blocked `read()` returns immediately.
//!
//! ## Windows
//! 1. On connect, grab the raw `SOCKET` handle via `as_raw_socket()`.
//! 2. On Ctrl+C, call `closesocket()` on that handle -- the blocked `recv()` returns immediately.
//! 3. An `AtomicBool` flag prevents the transport from double-closing on drop.
//!
//! No polling, no extra threads.

use std::fmt;
use std::io::{Read, Write};
use std::mem::ManuallyDrop;
#[cfg(unix)]
use std::net::Shutdown;
use std::net::{SocketAddr, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

#[cfg(windows)]
use std::os::windows::io::AsRawSocket;

use nu_protocol::HandlerGuard;
use ureq::Error;
use ureq::unversioned::transport::{
    Buffers, ConnectionDetails, Connector, LazyBuffers, NextTimeout, Transport,
};

/// Callback invoked when a connection is established.
/// Returns a guard that keeps the interrupt handler registered and a flag
/// indicating whether the socket has been closed by the interrupt handler.
pub type OnConnect =
    Arc<dyn Fn(&TcpStream) -> Option<(HandlerGuard, Arc<AtomicBool>)> + Send + Sync>;

/// Connector for interruptible TCP sockets.
pub struct InterruptibleTcpConnector {
    on_connect: Option<OnConnect>,
}

impl InterruptibleTcpConnector {
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

        Ok(Some(InterruptibleTcpTransport::new(
            stream, buffers, guard, closed,
        )))
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
/// The stream is wrapped in `ManuallyDrop` so that on Windows, if the interrupt handler
/// has already closed the socket via `closesocket()`, we skip the drop to avoid
/// double-close. The `closed` flag coordinates this.
pub struct InterruptibleTcpTransport {
    stream: ManuallyDrop<TcpStream>,
    buffers: LazyBuffers,
    timeout_write: Option<Duration>,
    timeout_read: Option<Duration>,
    closed: Arc<AtomicBool>,
    _guard: Option<HandlerGuard>,
}

impl InterruptibleTcpTransport {
    pub fn new(
        stream: TcpStream,
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

impl Drop for InterruptibleTcpTransport {
    fn drop(&mut self) {
        if !self.closed.swap(true, Ordering::AcqRel) {
            // SAFETY: We're in Drop, stream won't be used after this.
            unsafe { ManuallyDrop::drop(&mut self.stream) };
        }
        // else: interrupt handler already closed the socket, skip to avoid double-close
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
        // Match ureq's normalize_would_block behavior: WouldBlock -> TimedOut
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
        maybe_update_timeout(
            timeout,
            &mut self.timeout_read,
            &self.stream,
            TcpStream::set_read_timeout,
        )?;

        let input = self.buffers.input_append_buf();
        // Match ureq's normalize_would_block behavior: WouldBlock -> TimedOut
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

/// Create an `OnConnect` callback that registers a signal handler to interrupt the socket.
///
/// On Unix, the handler calls `shutdown()` via a cloned handle.
/// On Windows, the handler calls `closesocket()` on the original socket handle.
pub fn make_on_connect(handlers: &nu_protocol::Handlers) -> OnConnect {
    let handlers = handlers.clone();
    Arc::new(move |stream: &TcpStream| {
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
            let raw = stream.as_raw_socket() as usize;
            handlers
                .register(Box::new(move |action| {
                    if matches!(action, nu_protocol::SignalAction::Interrupt)
                        && !closed_clone.swap(true, Ordering::AcqRel)
                    {
                        // SAFETY: We close the socket exactly once (swap ensures this).
                        // The blocked recv() on the I/O thread returns immediately.
                        unsafe {
                            windows::Win32::Networking::WinSock::closesocket(
                                windows::Win32::Networking::WinSock::SOCKET(raw),
                            );
                        }
                    }
                }))
                .ok()?
        };

        // On Unix closed_clone is unused, suppress warning
        #[cfg(unix)]
        drop(closed_clone);

        Some((guard, closed))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::{Handlers, SignalAction};
    use std::io::Write;
    use std::net::TcpListener;
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    fn test_interrupt_unblocks_read() {
        // Start a server that accepts connections but delays sending data
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let server_thread = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            // Wait longer than our test timeout before sending anything
            thread::sleep(Duration::from_secs(10));
            let _ = stream.write_all(b"delayed response");
        });

        let handlers = Handlers::new();
        let on_connect = make_on_connect(&handlers);

        let stream = TcpStream::connect(addr).unwrap();
        let (guard, closed) = on_connect(&stream).unwrap();
        let transport = InterruptibleTcpTransport::new(
            stream,
            LazyBuffers::new(8192, 8192),
            Some(guard),
            closed,
        );

        let handlers_clone = handlers.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(100));
            handlers_clone.run(SignalAction::Interrupt);
        });

        let start = Instant::now();
        let mut buf = [0u8; 1024];
        let result = std::io::Read::read(&mut &*transport.stream, &mut buf);
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

        drop(transport);
        drop(server_thread);
    }

    #[test]
    fn test_connector_creates_transport() {
        let connector = InterruptibleTcpConnector::new(None);
        let debug_str = format!("{connector:?}");
        assert!(debug_str.contains("InterruptibleTcpConnector"));
    }
}
