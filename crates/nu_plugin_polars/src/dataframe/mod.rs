use nu_protocol::{FutureSpanId, ShellError};

pub mod eager;
pub mod expressions;
pub mod lazy;
pub mod series;
pub mod stub;
mod utils;
pub mod values;

pub fn missing_flag_error(flag: &str, span: FutureSpanId) -> ShellError {
    ShellError::GenericError {
        error: format!("Missing flag: {flag}"),
        msg: "".into(),
        span: Some(span),
        help: None,
        inner: vec![],
    }
}
