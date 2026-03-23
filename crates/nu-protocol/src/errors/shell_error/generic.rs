use crate::{ShellError, Span};
use miette::Diagnostic;
use nu_utils::location::Location;
use std::{
    borrow::Cow,
    error::Error,
    fmt::{self, Display},
};

const DEFAULT_CODE: &str = "nu::shell::error";

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct GenericError {
    pub code: Cow<'static, str>,
    pub error: Cow<'static, str>,
    pub msg: Cow<'static, str>,
    pub source: SpanOrLocation,
    pub help: Option<Cow<'static, str>>,
    pub inner: Vec<ShellError>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SpanOrLocation {
    Span(Span),
    Location(String),
}

impl GenericError {
    pub fn new(
        error: impl Into<Cow<'static, str>>,
        msg: impl Into<Cow<'static, str>>,
        span: Span,
    ) -> Self {
        Self {
            code: DEFAULT_CODE.into(),
            error: error.into(),
            msg: msg.into(),
            source: SpanOrLocation::Span(span),
            help: None,
            inner: Vec::new(),
        }
    }

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
            source: SpanOrLocation::Location(location.to_string()),
            help: None,
            inner: Vec::new(),
        }
    }

    pub fn with_code(self, code: impl Into<Cow<'static, str>>) -> Self {
        Self {
            code: code.into(),
            ..self
        }
    }

    pub fn with_help(self, help: impl Into<Cow<'static, str>>) -> Self {
        Self {
            help: Some(help.into()),
            ..self
        }
    }

    pub fn with_inner(self, inner: impl IntoIterator<Item = ShellError>) -> Self {
        Self {
            inner: inner.into_iter().collect(),
            ..self
        }
    }
}

impl Display for GenericError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let GenericError { error, msg, .. } = self;
        write!(f, "{error}: {msg}")
    }
}

impl Error for GenericError {}

impl Diagnostic for GenericError {
    fn code<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
        Some(Box::new(self.code.as_ref()))
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        let span = match &self.source {
            SpanOrLocation::Span(span) => (*span).into(),
            SpanOrLocation::Location(location) => miette::SourceSpan::new(0.into(), location.len()),
        };

        let label = miette::LabeledSpan::new_with_span(Some(self.msg.to_string()), span);
        Some(Box::new(std::iter::once(label)))
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        match &self.source {
            SpanOrLocation::Span(_) => None,
            SpanOrLocation::Location(location) => Some(location as &dyn miette::SourceCode),
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
}
