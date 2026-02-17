use nu_protocol::ShellError;

pub(crate) fn map_arboard_err_to_shell(err: arboard::Error) -> ShellError {
    ShellError::GenericError {
        error: "Clipboard error".into(),
        msg: err.to_string(),
        span: None,
        help: None,
        inner: vec![],
    }
}
