use super::ShellError;
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

#[test]
fn test_bridge_io_error_roundtrip() {
    let shell_error = ShellError::GenericError {
        error: "some error".into(),
        msg: "some message".into(),
        span: None,
        help: None,
        inner: vec![],
    };

    let bridge = ShellErrorBridge(shell_error);
    let io_error = std::io::Error::from(bridge.clone());
    let bridge_again = ShellErrorBridge::try_from(io_error).unwrap();
    assert_eq!(bridge.0, bridge_again.0);
}
