use nu_protocol::{ShellError, Span};

pub mod command;
mod utils;
pub mod values;
mod cloud;

pub fn missing_flag_error(flag: &str, span: Span) -> ShellError {
    ShellError::GenericError {
        error: format!("Missing flag: {flag}"),
        msg: "".into(),
        span: Some(span),
        help: None,
        inner: vec![],
    }
}
