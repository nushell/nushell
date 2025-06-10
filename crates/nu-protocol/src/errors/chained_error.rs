use super::shell_error::ShellError;
use crate::Span;
use miette::{LabeledSpan, Severity, SourceCode};
use thiserror::Error;

/// An error struct that contains source errors.
///
/// However, it's a bit special; if the error is constructed for the first time using
/// [`ChainedError::new`], it will behave the same as the single source error.
///
/// If it's constructed nestedly using [`ChainedError::new_chained`], it will treat all underlying errors as related.
///
/// For a usage example, please check [`ShellError::into_chainned`].
#[derive(Debug, Clone, PartialEq, Error)]
pub struct ChainedError {
    first: bool,
    pub(crate) sources: Vec<ShellError>,
    span: Span,
}

impl std::fmt::Display for ChainedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.first {
            write!(f, "{}", self.sources[0])
        } else {
            write!(f, "oops")
        }
    }
}

impl ChainedError {
    pub fn new(source: ShellError, span: Span) -> Self {
        Self {
            first: true,
            sources: vec![source],
            span,
        }
    }

    pub fn new_chained(sources: Self, span: Span) -> Self {
        Self {
            first: false,
            sources: vec![ShellError::ChainedError(sources)],
            span,
        }
    }
}

impl miette::Diagnostic for ChainedError {
    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn miette::Diagnostic> + 'a>> {
        if self.first {
            self.sources[0].related()
        } else {
            Some(Box::new(self.sources.iter().map(|s| s as _)))
        }
    }

    fn code<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        if self.first {
            self.sources[0].code()
        } else {
            Some(Box::new("chained_error"))
        }
    }

    fn severity(&self) -> Option<Severity> {
        if self.first {
            self.sources[0].severity()
        } else {
            None
        }
    }

    fn help<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        if self.first {
            self.sources[0].help()
        } else {
            None
        }
    }

    fn url<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        if self.first {
            self.sources[0].url()
        } else {
            None
        }
    }

    fn labels<'a>(&'a self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + 'a>> {
        if self.first {
            self.sources[0].labels()
        } else {
            Some(Box::new(
                vec![LabeledSpan::new_with_span(
                    Some("error happened when running this".to_string()),
                    self.span,
                )]
                .into_iter(),
            ))
        }
    }

    // Finally, we redirect the source_code method to our own source.
    fn source_code(&self) -> Option<&dyn SourceCode> {
        if self.first {
            self.sources[0].source_code()
        } else {
            None
        }
    }

    fn diagnostic_source(&self) -> Option<&dyn miette::Diagnostic> {
        if self.first {
            self.sources[0].diagnostic_source()
        } else {
            None
        }
    }
}
