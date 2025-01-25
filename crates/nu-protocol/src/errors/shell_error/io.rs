use miette::{Diagnostic, LabeledSpan, SourceSpan};
use std::{fmt::Display, path::PathBuf};
use thiserror::Error;

use crate::Span;

use super::{location::Location, ShellError};

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

    /// The precise location in the Rust code where the error originated.
    ///
    /// This field is particularly useful for debugging errors that stem from the Rust
    /// implementation rather than user-provided Nushell code.
    /// The original [`Location`] is converted to a string to more easily report the error
    /// attributing the location.
    ///
    /// This value is only used if `span` is [`Span::unknown()`] as most of the time we want to
    /// refer to user code than the Rust code.
    pub location: Option<String>,

    /// An intentionally unused private field to prevent direct construction.
    ///
    /// This field ensures that the struct can only be created through the accompanying constructors,
    /// motivating construction of this type with proper fields.
    /// 
    /// Public fields can still be modified if required but this requires more work and is therefore 
    /// probably not done.
    ///
    /// This field is marked with `#[doc(hidden)]` to hide it from the public API.
    /// It is purely a zero-sized marker field and has no runtime cost.
    #[doc(hidden)]
    _force_constructor: (),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Diagnostic)]
pub enum ErrorKind {
    Std(std::io::ErrorKind),
    // TODO: in Rust 1.83 this can be std::io::ErrorKind::NotADirectory
    NotADirectory,
    NotAFile,
    // TODO: in Rust 1.83 this can be std::io::ErrorKind::IsADirectory
    IsADirectory,
}

impl IoError {
    /// Creates a new [`IoError`] with the given kind, span, and optional path.
    ///
    /// This constructor should be used in all cases where the combination of the error kind, span,
    /// and path provides enough information to describe the error clearly.
    /// For example, errors like "File not found" or "Permission denied" are typically
    /// self-explanatory when paired with the file path and the location in user-provided
    /// Nushell code (`span`).
    ///
    /// # Constraints
    /// If `span` is unknown, use:
    /// - `new_internal` if no path is available.
    /// - `new_internal_with_path` if a path is available.
    pub fn new(kind: impl Into<ErrorKind>, span: Span, path: impl Into<Option<PathBuf>>) -> Self {
        let path = path.into();
        if span == Span::unknown() {
            debug_assert!(path.is_some(), "for unknown spans with paths, use `new_internal_with_path`");
            debug_assert!(path.is_none(), "for unknown spans without paths, use `new_internal`");
        }
        
        Self {
            kind: kind.into(),
            span,
            path: path.into(),
            additional_context: None,
            location: None,
            _force_constructor: (),
        }
    }

    /// Creates a new [`IoError`] with additional context.
    ///
    /// Use this constructor when the error kind, span, and path are not sufficient to fully
    /// explain the error, and additional context can provide meaningful details.
    /// Avoid redundant context (e.g., "Permission denied" for an error kind of
    /// [`ErrorKind::PermissionDenied`](std::io::ErrorKind::PermissionDenied)).
    /// 
    /// # Constraints
    /// If `span` is unknown, use:
    /// - `new_internal` if no path is available.
    /// - `new_internal_with_path` if a path is available.
    pub fn new_with_additional_context(
        kind: impl Into<ErrorKind>,
        span: Span,
        path: impl Into<Option<PathBuf>>,
        additional_context: impl ToString,
    ) -> Self {
        let path = path.into();
        if span == Span::unknown() {
            debug_assert!(path.is_some(), "for unknown spans with paths, use `new_internal_with_path`");
            debug_assert!(path.is_none(), "for unknown spans without paths, use `new_internal`");
        }

        Self {
            kind: kind.into(),
            span,
            path: path,
            additional_context: Some(additional_context.to_string()),
            location: None,
            _force_constructor: ()
        }
    }

    /// Creates a new [`IoError`] for internal I/O errors without a user-provided span or path.
    ///
    /// This constructor is intended for internal errors in the Rust implementation that still need
    /// to be reported to the end user.
    /// Since these errors are not tied to user-provided Nushell code, they generally have no
    /// meaningful span or path.
    ///
    /// Instead, these errors provide:
    /// - `additional_context`:
    ///   Details about what went wrong internally.
    /// - `location`:
    ///   The location in the Rust code where the error occurred, allowing us to trace and debug
    ///   the issue.
    ///   Use the [`nu_protocol::location!`](crate::location) macro to generate the location
    ///   information.
    ///
    /// # Examples
    /// ```rust
    /// use nu_protocol::location;
    ///
    /// let error = IoError::new_internal(
    ///     ErrorKind::UnexpectedEof,
    ///     "Failed to read from buffer",
    ///     location!(),
    /// );
    /// ```
    pub fn new_internal(
        kind: impl Into<ErrorKind>,
        additional_context: impl ToString,
        location: Location,
    ) -> Self {
        Self {
            kind: kind.into(),
            span: Span::unknown(),
            path: None,
            additional_context: Some(additional_context.to_string()),
            location: Some(location.to_string()),
            _force_constructor: ()
        }
    }

    /// Creates a new `IoError` for internal I/O errors with a specific path.
    ///
    /// This constructor is similar to [`new_internal`] but also includes a file or directory 
    /// path relevant to the error. Use this function in rare cases where an internal error 
    /// involves a specific path, and the combination of path and additional context is helpful.
    /// 
    /// # Examples
    /// ```rust
    /// use nu_protocol::location;
    /// 
    /// let error = IoError::new_internal_with_path(
    ///     ErrorKind::PermissionDenied,
    ///     Some("/root/private-file".into()),
    ///     "Access denied while attempting to read the file",
    ///     location!(),
    /// );
    /// ```
    pub fn new_internal_with_path(
        kind: impl Into<ErrorKind>,
        additional_context: impl ToString,
        location: Location,
        path: PathBuf,
    ) -> Self {
        Self {
            kind: kind.into(),
            span: Span::unknown(),
            path: path.into(),
            additional_context: Some(additional_context.to_string()),
            location: Some(location.to_string()),
            _force_constructor: (),
        }
    }
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::Std(error_kind) => {
                let msg = error_kind.to_string();
                let (first, rest) = msg.split_at(1);
                write!(f, "{}{}", first.to_uppercase(), rest)
            },
            ErrorKind::NotADirectory => write!(f, "Not a directory"),
            ErrorKind::NotAFile => write!(f, "Not a file"),
            ErrorKind::IsADirectory => write!(f, "Is a directory"),
        }
    }
}

impl std::error::Error for ErrorKind {}

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
            ErrorKind::IsADirectory => code.push_str("is_a_directory"),
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
        let span_is_unknown = self.span == Span::unknown();
        let span = match (span_is_unknown, self.location.as_ref()) {
            (true, None) => return None,
            (false, _) => SourceSpan::from(self.span),
            (true, Some(location)) => SourceSpan::new(0.into(), location.len()),
        };

        let label = LabeledSpan::new_with_span(self.additional_context.clone(), span);
        Some(Box::new(std::iter::once(label)))
    }

    fn diagnostic_source(&self) -> Option<&dyn Diagnostic> {
        Some(&self.kind as &dyn Diagnostic)
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        let span_is_unknown = self.span == Span::unknown();
        match (span_is_unknown, self.location.as_ref()) {
            (true, None) | (false, _) => None,
            (true, Some(location)) => Some(location as &dyn miette::SourceCode),
        }
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
