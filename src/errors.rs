use crate::prelude::*;
use crate::Value;
use derive_new::new;

#[derive(Debug, new)]
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
