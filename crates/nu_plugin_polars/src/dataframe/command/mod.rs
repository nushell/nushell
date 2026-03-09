use nu_protocol::{ShellError, Span};

pub mod aggregation;
pub mod boolean;
pub mod computation;
pub mod core;
pub mod data;
pub mod datetime;
pub mod index;
pub mod integer;
pub mod list;
pub mod selector;
pub mod string;
pub mod stub;

pub fn required_flag(flag: &str, span: Span) -> ShellError {
    ShellError::GenericError {
        error: format!("Flag {flag} is required."),
        msg: "".into(),
        span: Some(span),
        help: None,
        inner: vec![],
    }
}
