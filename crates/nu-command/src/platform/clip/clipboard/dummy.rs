use super::provider::Clipboard;
use nu_protocol::ShellError;

pub(crate) struct DummyClipboard;

impl DummyClipboard {
    pub fn new() -> Self {
        Self
    }
}

impl Clipboard for DummyClipboard {
    fn copy_text(&self, _text: &str) -> Result<(), ShellError> {
        Err(ShellError::GenericError {
            error: "Clipboard not supported".into(),
            msg: "Clipboard is not supported on this platform".into(),
            span: None,
            help: Some("nushell needs clipboard support for this platform to be added".into()),
            inner: vec![],
        })
    }

    fn get_text(&self) -> Result<String, ShellError> {
        Err(ShellError::GenericError {
            error: "Clipboard not supported".into(),
            msg: "Clipboard is not supported on this platform".into(),
            span: None,
            help: Some("nushell needs clipboard support for this platform to be added".into()),
            inner: vec![],
        })
    }
}
