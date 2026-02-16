use std::fmt;

/// Error type for serialization and deserialization.
#[derive(Debug)]
pub struct Error {
    message: String,
}

impl Error {
    pub fn new(msg: impl fmt::Display) -> Self {
        Error {
            message: msg.to_string(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for Error {}

impl serde::ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::new(msg)
    }
}

impl serde::de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::new(msg)
    }
}

impl From<Error> for crate::ShellError {
    fn from(e: Error) -> Self {
        crate::ShellError::GenericError {
            error: "nu_protocol::serde error".into(),
            msg: e.message,
            span: None,
            help: None,
            inner: vec![],
        }
    }
}
