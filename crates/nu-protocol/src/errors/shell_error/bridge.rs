#[cfg(test)]
use super::generic::GenericError;
use super::{ShellError, io::IoError};
use crate::Span;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A bridge for transferring a [`ShellError`] between Nushell or similar processes.
///
/// This newtype encapsulates a [`ShellError`] to facilitate its transfer between Nushell processes
/// or processes with similar behavior.
/// By defining this type, we eliminate ambiguity about what is being transferred and avoid the
/// need to implement [`From<io::Error>`](From) and [`Into<io::Error>`](Into) directly on
/// `ShellError`.
#[derive(Debug, Clone, PartialEq, Error, Serialize, Deserialize)]
#[error("{0}")]
pub struct ShellErrorBridge(pub ShellError);

impl ShellErrorBridge {
    /// Converts an I/O error from reading a stream into a [`ShellError`]. A [`ShellError`] packed
    /// in a bridge comes back out as is; anything else becomes an [`IoError`] at `span`.
    ///
    /// Anything that reads a stream should use this instead of [`IoError::new`], which would lose
    /// the error in the bridge, even if that's an `exit`.
    pub fn unpack(err: std::io::Error, span: Span) -> ShellError {
        match Self::try_from(err) {
            Ok(Self(err)) => err,
            Err(err) => IoError::new(err, span, None).into(),
        }
    }
}

impl TryFrom<std::io::Error> for ShellErrorBridge {
    type Error = std::io::Error;

    fn try_from(value: std::io::Error) -> Result<Self, Self::Error> {
        let kind = value.kind();
        value
            .downcast()
            .inspect(|_| debug_assert_eq!(kind, std::io::ErrorKind::Other))
    }
}

impl From<ShellErrorBridge> for std::io::Error {
    fn from(value: ShellErrorBridge) -> Self {
        std::io::Error::other(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_io_error_roundtrip() {
        let shell_error =
            ShellError::Generic(GenericError::new_internal("some error", "some message"));

        let bridge = ShellErrorBridge(shell_error);
        let io_error = std::io::Error::from(bridge.clone());
        let bridge_again = ShellErrorBridge::try_from(io_error).unwrap();
        assert_eq!(bridge.0, bridge_again.0);
    }
}
