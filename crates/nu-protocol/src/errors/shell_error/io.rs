use miette::{Diagnostic, LabeledSpan, SourceSpan};
use std::{
    error::Error as StdError,
    fmt::{self, Display, Formatter},
    path::{Path, PathBuf},
};
use thiserror::Error;

use crate::Span;

use super::{location::Location, ShellError};

/// Represents an I/O error in the [`ShellError::Io`] variant.
///
/// This is the central I/O error for the [`ShellError::Io`] variant.
/// It represents all I/O errors by encapsulating [`ErrorKind`], an extension of
/// [`std::io::ErrorKind`].
/// The `span` indicates where the error occurred in user-provided code.
/// If the error is not tied to user-provided code, the `location` refers to the precise point in
/// the Rust code where the error originated.
/// The optional `path` provides the file or directory involved in the error.
/// If [`ErrorKind`] alone doesn't provide enough detail, additional context can be added to clarify
/// the issue.
///
/// For handling user input errors (e.g., commands), prefer using [`new`](Self::new).
/// Alternatively, use the [`factory`](Self::factory) method to simplify error creation in repeated
/// contexts.
/// For internal errors, use [`new_internal`](Self::new_internal) to include the location in Rust
/// code where the error originated.
///
/// # Examples
///
/// ## User Input Error
/// ```rust
/// # use nu_protocol::shell_error::io::{IoError, ErrorKind};
/// # use nu_protocol::Span;
/// use std::path::PathBuf;
///
/// # let span = Span::test_data();
/// let path = PathBuf::from("/some/missing/file");
/// let error = IoError::new(std::io::ErrorKind::NotFound, span, path);
/// println!("Error: {:?}", error);
/// ```
///
/// ## Internal Error
/// ```rust
/// # use nu_protocol::shell_error::io::{IoError, ErrorKind};
//  #
/// let error = IoError::new_internal(
///     std::io::ErrorKind::UnexpectedEof,
///     "Failed to read data from buffer",
///     nu_protocol::location!()
/// );
/// println!("Error: {:?}", error);
/// ```
///
/// ## Using the Factory Method
/// ```rust
/// # use nu_protocol::shell_error::io::{IoError, ErrorKind};
/// # use nu_protocol::{Span, ShellError};
/// use std::path::PathBuf;
///
/// # fn should_return_err() -> Result<(), ShellError> {
/// # let span = Span::new(50, 60);
/// let path = PathBuf::from("/some/file");
/// let from_io_error = IoError::factory(span, Some(path.as_path()));
///
/// let content = std::fs::read_to_string(&path).map_err(from_io_error)?;
/// # Ok(())
/// # }
/// #
/// # assert!(should_return_err().is_err());
/// ```
///
/// # ShellErrorBridge
///
/// The [`ShellErrorBridge`](super::bridge::ShellErrorBridge) struct is used to contain a
/// [`ShellError`] inside a [`std::io::Error`].
/// This allows seamless transfer of `ShellError` instances where `std::io::Error` is expected.
/// When a `ShellError` needs to be packed into an I/O context, use this bridge.
/// Similarly, when handling an I/O error that is expected to contain a `ShellError`,
/// use the bridge to unpack it.
///
/// This approach ensures clarity about where such container transfers occur.
/// All other I/O errors should be handled using the provided constructors for `IoError`.
/// This way, the code explicitly indicates when and where a `ShellError` transfer might happen.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
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
    pub additional_context: Option<AdditionalContext>,

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Diagnostic)]
pub enum ErrorKind {
    Std(std::io::ErrorKind),
    NotAFile,
    // use these variants in cases where we know precisely whether a file or directory was expected
    FileNotFound,
    DirectoryNotFound,
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Diagnostic)]
#[error("{0}")]
pub struct AdditionalContext(String);

impl From<String> for AdditionalContext {
    fn from(value: String) -> Self {
        AdditionalContext(value)
    }
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
            debug_assert!(
                path.is_some(),
                "for unknown spans with paths, use `new_internal_with_path`"
            );
            debug_assert!(
                path.is_none(),
                "for unknown spans without paths, use `new_internal`"
            );
        }

        Self {
            kind: kind.into(),
            span,
            path,
            additional_context: None,
            location: None,
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
            debug_assert!(
                path.is_some(),
                "for unknown spans with paths, use `new_internal_with_path`"
            );
            debug_assert!(
                path.is_none(),
                "for unknown spans without paths, use `new_internal`"
            );
        }

        Self {
            kind: kind.into(),
            span,
            path,
            additional_context: Some(additional_context.to_string().into()),
            location: None,
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
    /// use nu_protocol::shell_error::io::IoError;
    ///
    /// let error = IoError::new_internal(
    ///     std::io::ErrorKind::UnexpectedEof,
    ///     "Failed to read from buffer",
    ///     nu_protocol::location!(),
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
            additional_context: Some(additional_context.to_string().into()),
            location: Some(location.to_string()),
        }
    }

    /// Creates a new `IoError` for internal I/O errors with a specific path.
    ///
    /// This constructor is similar to [`new_internal`](Self::new_internal) but also includes a
    /// file or directory path relevant to the error.
    /// Use this function in rare cases where an internal error involves a specific path, and the
    /// combination of path and additional context is helpful.
    ///
    /// # Examples
    /// ```rust
    /// use nu_protocol::shell_error::io::IoError;
    /// use std::path::PathBuf;
    ///
    /// let error = IoError::new_internal_with_path(
    ///     std::io::ErrorKind::NotFound,
    ///     "Could not find special file",
    ///     nu_protocol::location!(),
    ///     PathBuf::from("/some/file"),
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
            additional_context: Some(additional_context.to_string().into()),
            location: Some(location.to_string()),
        }
    }

    /// Creates a factory closure for constructing [`IoError`] instances from [`std::io::Error`] values.
    ///
    /// This method is particularly useful when you need to handle multiple I/O errors which all
    /// take the same span and path.
    /// Instead of calling `.map_err(|err| IoError::new(err.kind(), span, path))` every time, you
    /// can create the factory closure once and pass that into `.map_err`.
    pub fn factory<'p, P>(span: Span, path: P) -> impl Fn(std::io::Error) -> Self + use<'p, P>
    where
        P: Into<Option<&'p Path>>,
    {
        let path = path.into();
        move |err: std::io::Error| IoError::new(err.kind(), span, path.map(PathBuf::from))
    }
}

impl StdError for IoError {}
impl Display for IoError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.kind {
            ErrorKind::Std(std::io::ErrorKind::NotFound) => write!(f, "Not found"),
            ErrorKind::FileNotFound => write!(f, "File not found"),
            ErrorKind::DirectoryNotFound => write!(f, "Directory not found"),
            _ => write!(f, "I/O error"),
        }
    }
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::Std(std::io::ErrorKind::NotFound) => write!(f, "Not found"),
            ErrorKind::Std(error_kind) => {
                let msg = error_kind.to_string();
                let (first, rest) = msg.split_at(1);
                write!(f, "{}{}", first.to_uppercase(), rest)
            }
            ErrorKind::NotAFile => write!(f, "Not a file"),
            ErrorKind::FileNotFound => write!(f, "File not found"),
            ErrorKind::DirectoryNotFound => write!(f, "Directory not found"),
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
                kind => code.push_str(&kind.to_string().to_lowercase().replace(" ", "_")),
            },
            ErrorKind::NotAFile => code.push_str("not_a_file"),
            ErrorKind::FileNotFound => code.push_str("file_not_found"),
            ErrorKind::DirectoryNotFound => code.push_str("directory_not_found"),
        }

        Some(Box::new(code))
    }

    fn help<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        let make_msg = |path: &Path| {
            let path = format!("'{}'", path.display());
            match self.kind {
                ErrorKind::NotAFile => format!("{path} is not a file"),
                ErrorKind::Std(std::io::ErrorKind::NotFound)
                | ErrorKind::FileNotFound
                | ErrorKind::DirectoryNotFound => format!("{path} does not exist"),
                _ => format!("The error occurred at {path}"),
            }
        };

        self.path
            .as_ref()
            .map(|path| make_msg(path))
            .map(|s| Box::new(s) as Box<dyn std::fmt::Display>)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        let span_is_unknown = self.span == Span::unknown();
        let span = match (span_is_unknown, self.location.as_ref()) {
            (true, None) => return None,
            (false, _) => SourceSpan::from(self.span),
            (true, Some(location)) => SourceSpan::new(0.into(), location.len()),
        };

        let label = LabeledSpan::new_with_span(Some(self.kind.to_string()), span);
        Some(Box::new(std::iter::once(label)))
    }

    fn diagnostic_source(&self) -> Option<&dyn Diagnostic> {
        self.additional_context
            .as_ref()
            .map(|ctx| ctx as &dyn Diagnostic)
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

/// More specific variants of [`NotFound`](std::io::ErrorKind).
///
/// Use these to define how a `NotFound` error maps to our custom [`ErrorKind`].
pub enum NotFound {
    /// Map into [`FileNotFound`](ErrorKind::FileNotFound).
    File,
    /// Map into [`DirectoryNotFound`](ErrorKind::DirectoryNotFound).
    Directory,
}

/// Extension trait for working with [`std::io::ErrorKind`].
pub trait ErrorKindExt {
    /// Map [`NotFound`](std::io::ErrorKind) variants into more precise variants.
    ///
    /// The OS doesn't know when an entity was not found whether it was meant to be a file or a
    /// directory or something else.
    /// But sometimes we, the application, know what we expect and with this method, we can further
    /// specify it.
    ///
    /// # Examples
    /// Reading a file.
    /// If the file isn't found, return [`FileNotFound`](ErrorKind::FileNotFound).
    /// ```rust
    /// # use nu_protocol::{
    /// #     shell_error::io::{ErrorKind, ErrorKindExt, IoError, NotFound},
    /// #     ShellError, Span,
    /// # };
    /// # use std::{fs, path::PathBuf};
    /// #
    /// # fn example() -> Result<(), ShellError> {
    /// #     let span = Span::test_data();
    /// let a_file = PathBuf::from("scripts/ellie.nu");
    /// let ellie = fs::read_to_string(&a_file).map_err(|err| {
    ///     ShellError::Io(IoError::new(
    ///         err.kind().not_found_as(NotFound::File),
    ///         span,
    ///         a_file,
    ///     ))
    /// })?;
    /// #     Ok(())
    /// # }
    /// #
    /// # assert!(matches!(
    /// #     example(),
    /// #     Err(ShellError::Io(IoError {
    /// #         kind: ErrorKind::FileNotFound,
    /// #         ..
    /// #     }))
    /// # ));
    /// ```
    fn not_found_as(self, kind: NotFound) -> ErrorKind;
}

impl ErrorKindExt for std::io::ErrorKind {
    fn not_found_as(self, kind: NotFound) -> ErrorKind {
        match (kind, self) {
            (NotFound::File, Self::NotFound) => ErrorKind::FileNotFound,
            (NotFound::Directory, Self::NotFound) => ErrorKind::DirectoryNotFound,
            _ => ErrorKind::Std(self),
        }
    }
}

impl ErrorKindExt for ErrorKind {
    fn not_found_as(self, kind: NotFound) -> ErrorKind {
        match self {
            Self::Std(std_kind) => std_kind.not_found_as(kind),
            _ => self,
        }
    }
}
