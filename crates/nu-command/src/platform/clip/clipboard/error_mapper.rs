use nu_protocol::{ShellError, shell_error::generic::GenericError};

pub(crate) fn map_arboard_err_to_shell(err: arboard::Error) -> ShellError {
    ShellError::Generic(GenericError::new_internal(
        "Clipboard error",
        err.to_string(),
    ))
}
