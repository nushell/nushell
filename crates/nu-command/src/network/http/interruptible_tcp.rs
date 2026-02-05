//! Interruptible TCP connector for ureq HTTP client.
//!
//! This module provides a TCP transport implementation that can be interrupted
//! when the user presses Ctrl+C by storing a cloned socket handle.

use std::fmt;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use ureq::Error;
use ureq::unversioned::transport::{
    Buffers, ConnectionDetails, Connector, LazyBuffers, NextTimeout, Transport,
};

use super::interruptible_stream::ActiveConnections;

/// Connector for interruptible TCP sockets.
///
/// This connector stores cloned socket handles in ActiveConnections,
/// allowing them to be shutdown when Ctrl+C is pressed.
#[derive(Debug)]
pub struct InterruptibleTcpConnector {
    active_connections: ActiveConnections,
}

impl InterruptibleTcpConnector {
    /// Create a new interruptible TCP connector.
    pub fn new(active_connections: ActiveConnections) -> Self {
        Self { active_connections }
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

        // Clone the stream and store for interruption
        if let Ok(clone) = stream.try_clone() {
            self.active_connections.add(clone);
        }

        let buffers = LazyBuffers::new(
            details.config.input_buffer_size(),
            details.config.output_buffer_size(),
        );

        Ok(Some(InterruptibleTcpTransport::new(stream, buffers)))
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
    let stream = if let Some(t) = timeout.not_zero() {
        TcpStream::connect_timeout(&addr, *t).map_err(|e| {
            if e.kind() == std::io::ErrorKind::TimedOut {
                Error::Timeout(timeout.reason)
            } else {
                Error::Io(e)
            }
        })?
    } else {
        TcpStream::connect(addr).map_err(Error::Io)?
    };

    // Set TCP_NODELAY for better latency
    let _ = stream.set_nodelay(true);

    Ok(stream)
}

/// Transport implementation for interruptible TCP sockets.
pub struct InterruptibleTcpTransport {
    stream: TcpStream,
    buffers: LazyBuffers,
    timeout_write: Option<Duration>,
    timeout_read: Option<Duration>,
}

impl InterruptibleTcpTransport {
    /// Create a new TCP transport.
    pub fn new(stream: TcpStream, buffers: LazyBuffers) -> Self {
        Self {
            stream,
            buffers,
            timeout_read: None,
            timeout_write: None,
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
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => Err(Error::Timeout(timeout.reason)),
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
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                return Err(Error::Timeout(timeout.reason))
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => 0,
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
