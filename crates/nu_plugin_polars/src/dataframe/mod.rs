use nu_protocol::{ShellError, Span};
use nu_utils::uformat;

pub mod command;
mod utils;
pub mod values;

pub fn missing_flag_error(flag: &str, span: Span) -> ShellError {
    ShellError::GenericError {
        error: uformat!("Missing flag: {flag}"),
        msg: "".into(),
        span: Some(span),
        help: None,
        inner: vec![],
    }
}
