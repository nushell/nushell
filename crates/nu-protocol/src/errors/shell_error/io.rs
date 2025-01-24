use miette::{Diagnostic, LabeledSpan};
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
    /// Only in very rare cases should [`std::io::ErrorKind::Other`] be used, make sure you provide
    /// `additional_context` to get useful errors in these cases.
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

#[derive(Debug, Clone, Copy, PartialEq, Error)]
pub enum ErrorKind {
    #[error("{0}")]
    Std(std::io::ErrorKind),
    // TODO: in Rust 1.83 this can be std::io::ErrorKind::NotADirectory
    #[error("not a directory")]
    NotADirectory,
    #[error("not a file")]
    NotAFile,
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
        let mut code = String::from("nu::shell::io::");
        match self.kind {
            ErrorKind::Std(error_kind) => match error_kind {
                std::io::ErrorKind::NotFound => code.push_str("not_found"),
                std::io::ErrorKind::PermissionDenied => code.push_str("permission_denied"),
                std::io::ErrorKind::ConnectionRefused => code.push_str("connection_refused"),
                std::io::ErrorKind::ConnectionReset => code.push_str("connection_reset"),
                std::io::ErrorKind::ConnectionAborted => code.push_str("connection_aborted"),
                std::io::ErrorKind::NotConnected => code.push_str("not_connected"),
                std::io::ErrorKind::AddrInUse => code.push_str("addr_in_use"),
                std::io::ErrorKind::AddrNotAvailable => code.push_str("addr_not_available"),
                std::io::ErrorKind::BrokenPipe => code.push_str("broken_pipe"),
                std::io::ErrorKind::AlreadyExists => code.push_str("already_exists"),
                std::io::ErrorKind::WouldBlock => code.push_str("would_block"),
                std::io::ErrorKind::InvalidInput => code.push_str("invalid_input"),
                std::io::ErrorKind::InvalidData => code.push_str("invalid_data"),
                std::io::ErrorKind::TimedOut => code.push_str("timed_out"),
                std::io::ErrorKind::WriteZero => code.push_str("write_zero"),
                std::io::ErrorKind::Interrupted => code.push_str("interrupted"),
                std::io::ErrorKind::Unsupported => code.push_str("unsupported"),
                std::io::ErrorKind::UnexpectedEof => code.push_str("unexpected_eof"),
                std::io::ErrorKind::OutOfMemory => code.push_str("out_of_memory"),
                std::io::ErrorKind::Other => code.push_str("other"),
                _ => code.push_str("unknown"),
            },
            ErrorKind::NotADirectory => code.push_str("not_a_directory"),
            ErrorKind::NotAFile => code.push_str("not_a_file"),
        }

        Some(Box::new(code))
    }

    fn help<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.path
            .as_ref()
            .map(|path| format!("The error occurred at '{}'", path.display()))
            .map(|s| Box::new(s) as Box<dyn std::fmt::Display>)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        let mut labels = vec![];
        labels.push(LabeledSpan::new_with_span(
            self.kind.to_string().into(),
            self.span,
        ));

        if let Some(ctx) = &self.additional_context {
            labels.push(LabeledSpan::new_with_span(
                ctx.to_string().into(),
                self.span,
            ));
        }

        Some(Box::new(labels.into_iter()))
    }
}

impl From<IoError> for ShellError {
    fn from(value: IoError) -> Self {
        ShellError::Io(value)
    }
}

impl From<IoError> for std::io::Error {
    fn from(value: IoError) -> Self {
        Self::new(value.kind.into(), value)
    }
}

impl From<std::io::ErrorKind> for ErrorKind {
    fn from(value: std::io::ErrorKind) -> Self {
        ErrorKind::Std(value)
    }
}

impl From<ErrorKind> for std::io::ErrorKind {
    fn from(value: ErrorKind) -> Self {
        match value {
            ErrorKind::Std(error_kind) => error_kind,
            _ => std::io::ErrorKind::Other,
        }
    }
}
