use miette::Diagnostic;
use std::path::PathBuf;
use thiserror::Error;

use crate::Span;

use super::ShellError;

#[derive(Debug, Clone, Error, PartialEq)]
#[error("I/O error")]
pub struct IoError {
    /// The type of the underlying I/O error.
    ///
    /// [`std::io::ErrorKind`] provides detailed context about the type of I/O error that occurred
    /// and is part of [`std::io::Error`].
    /// If a kind cannot be represented by it, consider adding a new variant to [`ErrorKind`].
    ///
    /// Only in very rare cases should [`std::io::ErrorKind::Other`] be used.
    pub kind: ErrorKind,

    /// The source location of the error.
    pub span: Span,

    /// The path related to the I/O error, if applicable.
    ///
    /// Many I/O errors involve a file or directory path, but operating system error messages
    /// often don't include the specific path.
    /// Setting this to [`Some`] allows users to see which path caused the error.
    pub path: Option<PathBuf>,

    /// Additional details to provide more context about the error.
    ///
    /// Only set this field if it adds meaningful context.
    /// If [`ErrorKind`] already contains all the necessary information, leave this as [`None`].
    pub additional_context: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    Std(std::io::ErrorKind),
    // TODO: in Rust 1.83 this can be std::io::ErrorKind::NotADirectory
    NotADirectory,
}

impl IoError {
    pub fn new(kind: impl Into<ErrorKind>, span: Span, path: impl Into<Option<PathBuf>>) -> Self {
        Self {
            kind: kind.into(),
            span,
            path: path.into(),
            additional_context: None,
        }
    }

    pub fn new_with_additional_context(
        kind: impl Into<ErrorKind>,
        span: Span,
        path: impl Into<Option<PathBuf>>,
        additional_context: impl ToString,
    ) -> Self {
        Self {
            kind: kind.into(),
            span,
            path: path.into(),
            additional_context: Some(additional_context.to_string()),
        }
    }
}

impl Diagnostic for IoError {
    fn code<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        todo!()
    }
}

impl From<IoError> for ShellError {
    fn from(value: IoError) -> Self {
        ShellError::Io(value)
    }
}

impl From<std::io::ErrorKind> for ErrorKind {
    fn from(value: std::io::ErrorKind) -> Self {
        ErrorKind::Std(value)
    }
}
