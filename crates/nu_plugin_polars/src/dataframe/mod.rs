use nu_protocol::{ShellError, Span};

pub mod aggregation;
pub mod boolean;
pub mod core;
pub mod data;
pub mod datetime;
pub mod expressions;
pub mod index;
pub mod integer;
pub mod macro_commands;
pub mod series;
pub mod stub;
mod utils;
pub mod values;

pub fn missing_flag_error(flag: &str, span: Span) -> ShellError {
    ShellError::GenericError {
        error: format!("Missing flag: {flag}"),
        msg: "".into(),
        span: Some(span),
        help: None,
        inner: vec![],
    }
}
