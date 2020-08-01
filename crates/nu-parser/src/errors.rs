use std::fmt::Debug;

/// A combination of an informative parse error, and what has been successfully parsed so far
#[derive(Debug)]
pub struct ParseError<T: Debug> {
    /// An informative cause for this parse error
    pub(crate) cause: nu_errors::ParseError,

    /// What has been successfully parsed, if anything
    pub(crate) partial: Option<T>,
}

pub type ParseResult<T> = Result<T, ParseError<T>>;

impl<T: Debug> From<ParseError<T>> for nu_errors::ShellError {
    fn from(e: ParseError<T>) -> Self {
        e.cause.into()
    }
}
