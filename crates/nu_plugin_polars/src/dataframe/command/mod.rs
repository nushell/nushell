use nu_protocol::{ShellError, Span, shell_error::generic::GenericError};

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
    ShellError::Generic(GenericError::new(
        format!("Flag {flag} is required."),
        "",
        span,
    ))
}
