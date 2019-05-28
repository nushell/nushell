#[allow(unused)]
use crate::prelude::*;
use serde_derive::Serialize;
use derive_new::new;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, new, Clone, Serialize)]
pub struct ShellError {
    title: String,
    error: Value,
}

impl ShellError {
    crate fn string(title: impl Into<String>) -> ShellError {
        ShellError::new(title.into(), Value::nothing())
    }

    crate fn copy_error(&self) -> ShellError {
        ShellError {
            title: self.title.clone(),
            error: self.error.copy(),
        }
    }

    crate fn description(&self) -> String {
        self.title.clone()
    }
}

impl std::fmt::Display for ShellError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", &self.title)
    }
}

impl std::error::Error for ShellError {}

impl std::convert::From<std::io::Error> for ShellError {
    fn from(input: std::io::Error) -> ShellError {
        ShellError {
            title: format!("{}", input),
            error: Value::nothing(),
        }
    }
}

impl std::convert::From<futures_sink::VecSinkError> for ShellError {
    fn from(_input: futures_sink::VecSinkError) -> ShellError {
        ShellError {
            title: format!("Unexpected Vec Sink Error"),
            error: Value::nothing(),
        }
    }
}

impl std::convert::From<subprocess::PopenError> for ShellError {
    fn from(input: subprocess::PopenError) -> ShellError {
        ShellError {
            title: format!("{}", input),
            error: Value::nothing(),
        }
    }
}

impl std::convert::From<nom::Err<(&str, nom::error::ErrorKind)>> for ShellError {
    fn from(input: nom::Err<(&str, nom::error::ErrorKind)>) -> ShellError {
        ShellError {
            title: format!("{:?}", input),
            error: Value::nothing(),
        }
    }
}
