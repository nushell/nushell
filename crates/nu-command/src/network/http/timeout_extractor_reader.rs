//! Ureq 3.0.12 converts timeout errors into std::io::ErrorKind::Other: <https://github.com/algesten/ureq/blob/3.0.12/src/error.rs#L193>
//! But Nushell infrastructure expects std::io::ErrorKind::Timeout when an operation times out.
//! This is an adapter that converts former into latter.

use std::io::Read;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Convert errors io errors with [std::io::ErrorKind::Other] and [ureq::Error::Timeout]
/// into io errors with [std::io::ErrorKind::Timeout]
pub struct UreqTimeoutExtractorReader<R> {
    pub r: R,
}

impl<R: Read> Read for UreqTimeoutExtractorReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.r.read(buf).map_err(|e| {
            // TODO: if-let chains when rust 1.88

            // ureq packages time-outs into "other"
            if e.kind() != std::io::ErrorKind::Other {
                return e;
            }
            let ureq_err = match e.downcast::<ureq::Error>() {
                Err(e) => return e,
                Ok(e) => e,
            };
            match ureq_err {
                ureq::Error::Timeout(..) => {
                    std::io::Error::new(std::io::ErrorKind::TimedOut, ureq_err)
                }
                // package it back
                e => std::io::Error::other(e),
            }
        })
    }
}

/// A handle to track whether a transport error occurred during reading.
#[derive(Clone)]
pub struct TransportErrorTracker {
    inner: Arc<AtomicBool>,
}

impl TransportErrorTracker {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn set_error(&self) {
        self.inner.store(true, Ordering::Relaxed);
    }

    pub fn has_error(&self) -> bool {
        self.inner.load(Ordering::Relaxed)
    }
}

/// A reader wrapper that, when `allow_errors` is true, converts transport-level
/// I/O errors (timeouts, connection refused, DNS failures) into EOF (Ok(0)).
/// This allows the stream to end gracefully with partial data instead of erroring.
/// Also tracks whether a transport error occurred via `TransportErrorTracker`.
pub struct AllowErrorsReader<R> {
    r: R,
    allow_errors: bool,
    tracker: Option<TransportErrorTracker>,
}

impl<R: Read> AllowErrorsReader<R> {
    pub fn new(r: R, allow_errors: bool) -> Self {
        Self {
            r,
            allow_errors,
            tracker: None,
        }
    }

    pub fn with_tracker(r: R, allow_errors: bool, tracker: TransportErrorTracker) -> Self {
        Self {
            r,
            allow_errors,
            tracker: Some(tracker),
        }
    }
}

impl<R: Read> Read for AllowErrorsReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.r.read(buf) {
            Ok(n) => Ok(n),
            Err(e) if self.allow_errors && is_transport_io_error(&e) => {
                if let Some(tracker) = &self.tracker {
                    tracker.set_error();
                }
                Ok(0)
            }
            Err(e) => Err(e),
        }
    }
}

/// Returns true if the I/O error represents a transport-level failure
/// (timeout, connection refused, DNS failure, etc.) that should be
/// swallowed when `--allow-errors` is set.
pub fn is_transport_io_error(e: &std::io::Error) -> bool {
    matches!(
        e.kind(),
        std::io::ErrorKind::TimedOut
            | std::io::ErrorKind::ConnectionRefused
            | std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::ConnectionAborted
            | std::io::ErrorKind::NotConnected
            | std::io::ErrorKind::AddrNotAvailable
    )
}
