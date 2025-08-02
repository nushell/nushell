//! Ureq 3.0.12 converts timeout errors into std::io::ErrorKind::Other: <https://github.com/algesten/ureq/blob/3.0.12/src/error.rs#L193>
//! But Nushell infrastructure expects std::io::ErrorKind::Timeout when an operation times out.
//! This is an adapter that converts former into latter.

use std::io::Read;

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
                e => std::io::Error::new(std::io::ErrorKind::Other, e),
            }
        })
    }
}
