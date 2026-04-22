use super::provider::Clipboard;
use nu_protocol::{ShellError, shell_error::generic::GenericError};

pub(crate) struct DummyClipboard;

impl DummyClipboard {
    pub fn new() -> Self {
        Self
    }
}

impl Clipboard for DummyClipboard {
    fn copy_text(&self, _text: &str) -> Result<(), ShellError> {
        Err(ShellError::Generic(
            GenericError::new_internal(
                "Clipboard not supported",
                "Clipboard is not supported on this platform",
            )
            .with_help("nushell needs clipboard support for this platform to be added"),
        ))
    }

    fn get_text(&self) -> Result<String, ShellError> {
        Err(ShellError::Generic(
            GenericError::new_internal(
                "Clipboard not supported",
                "Clipboard is not supported on this platform",
            )
            .with_help("nushell needs clipboard support for this platform to be added"),
        ))
    }
}
