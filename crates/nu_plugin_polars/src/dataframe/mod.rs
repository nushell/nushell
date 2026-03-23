use nu_protocol::{ShellError, Span, shell_error::generic::GenericError};

pub mod command;
mod utils;
pub mod values;

pub fn missing_flag_error(flag: &str, span: Span) -> ShellError {
    ShellError::Generic(GenericError::new(format!("Missing flag: {flag}"), "", span))
}
