use crate::{
    ShellError, Span,
    shell_error::{ErrorSite, ErrorSource},
};
use miette::Diagnostic;
use nu_utils::location::Location;
use std::{
    borrow::Cow,
    error::Error as StdError,
    fmt::{self, Display},
    sync::Arc,
};

/// Default code that [`GenericError`] is using as error code.
pub const DEFAULT_CODE: &str = "nu::shell::error";

/// Generic [`ShellError`].
///
/// This is a generic error for all cases where any of the variants of [`ShellError`] do not fit
/// and creating new variants is too niche.
/// Usually this should be created using [`new`](Self::new) or [`new_internal`](Self::new_internal)
/// if absolutely no span is available, try however to provide at least some span like `call.head`
/// inside a [`Command::run`](crate::engine::Command::run) context.
///
/// Using [`with_code`](Self::with_code), [`with_help`](Self::with_help),
/// [`with_inner`](Self::with_inner) can improve the error type making it more useful.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct GenericError {
    /// The diagnostic code for this error.
    ///
    /// Defaults to [`DEFAULT_CODE`].
    /// Use [`with_code`](Self::with_code) to override it.
    pub code: Cow<'static, str>,

    /// A short, user-facing title for the error.
    pub error: Cow<'static, str>,

    /// The message describing what went wrong.
    pub msg: Cow<'static, str>,

    /// The error origin: either a user span or an internal Rust location.
    pub site: ErrorSite,

    /// Optional additional guidance for the user.
    pub help: Option<Cow<'static, str>>,

    /// Related errors that provide more context.
    pub inner: Vec<ShellError>,

    /// Optional error source.
    pub source: Option<ErrorSource>,
}

impl GenericError {
    /// Creates a new [`GenericError`] tied to user input.
    ///
    /// The `error` is a short title, the `msg` provides details, and the `span`
    /// points to the user code that triggered the issue.
    #[track_caller]
    pub fn new(
        error: impl Into<Cow<'static, str>>,
        msg: impl Into<Cow<'static, str>>,
        span: Span,
    ) -> Self {
        // TODO: enable this at some point to find where unknown spans are passed around
        // debug_assert_ne!(
        //     span,
        //     Span::unknown(),
        //     "do not use `Span::unknown()` in a `GenericError::new`, prefer `GenericError::new_internal`"
        // );

        Self {
            code: DEFAULT_CODE.into(),
            error: error.into(),
            msg: msg.into(),
            site: ErrorSite::Span(span),
            help: None,
            inner: Vec::new(),
            source: None,
        }
    }

    /// Creates a new [`GenericError`] for internal errors without a user span.
    ///
    /// This records the Rust call site in the `source` so the error can be
    /// traced even when no user-facing span is available.
    #[track_caller]
    pub fn new_internal(
        error: impl Into<Cow<'static, str>>,
        msg: impl Into<Cow<'static, str>>,
    ) -> Self {
        let location = Location::caller();
        Self {
            code: DEFAULT_CODE.into(),
            error: error.into(),
            msg: msg.into(),
            site: ErrorSite::Location(location.to_string()),
            help: None,
            inner: Vec::new(),
            source: None,
        }
    }

    /// Creates a new [`GenericError`] for internal errors without a user span but a provided
    /// Rust location.
    ///
    /// Use this in places where a [`Location`] is already recorded and just needs to passed on,
    /// otherwise prefer [`new_internal`](Self::new_internal).
    pub fn new_internal_with_location(
        error: impl Into<Cow<'static, str>>,
        msg: impl Into<Cow<'static, str>>,
        location: impl Into<Location>,
    ) -> Self {
        Self {
            code: DEFAULT_CODE.into(),
            error: error.into(),
            msg: msg.into(),
            site: ErrorSite::Location(location.into().to_string()),
            help: None,
            inner: Vec::new(),
            source: None,
        }
    }

    /// Overrides the diagnostic code for this error.
    pub fn with_code(self, code: impl Into<Cow<'static, str>>) -> Self {
        Self {
            code: code.into(),
            ..self
        }
    }

    /// Adds user-facing help text for this error.
    pub fn with_help(self, help: impl Into<Cow<'static, str>>) -> Self {
        Self {
            help: Some(help.into()),
            ..self
        }
    }

    /// Attaches related errors that provide additional context.
    pub fn with_inner(self, inner: impl IntoIterator<Item = ShellError>) -> Self {
        Self {
            inner: inner.into_iter().collect(),
            ..self
        }
    }

    /// Attaches error source that can be used to render error chains.
    pub fn with_source(self, source: impl StdError + Send + Sync + 'static) -> Self {
        Self {
            source: Some(ErrorSource(Arc::new(source))),
            ..self
        }
    }
}

impl Display for GenericError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let GenericError { error, .. } = self;
        write!(f, "{error}")
    }
}

impl StdError for GenericError {}

impl Diagnostic for GenericError {
    fn code<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
        Some(Box::new(self.code.as_ref()))
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        let span = match &self.site {
            ErrorSite::Span(span) => (*span).into(),
            ErrorSite::Location(location) => miette::SourceSpan::new(0.into(), location.len()),
        };

        let label = miette::LabeledSpan::new_with_span(Some(self.msg.to_string()), span);
        Some(Box::new(std::iter::once(label)))
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        match &self.site {
            ErrorSite::Span(_) => None,
            ErrorSite::Location(location) => Some(location as &dyn miette::SourceCode),
        }
    }

    fn help<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
        self.help
            .as_ref()
            .map(|help| Box::new(help.as_ref()) as Box<dyn Display>)
    }

    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn Diagnostic> + 'a>> {
        match &self.inner.is_empty() {
            true => None,
            false => Some(Box::new(
                self.inner.iter().map(|err| err as &dyn Diagnostic),
            )),
        }
    }

    fn diagnostic_source(&self) -> Option<&dyn Diagnostic> {
        self.source.as_ref().map(|err| err as &dyn Diagnostic)
    }
}
