use super::shell_error::ShellError;
use crate::Span;
use miette::{Diagnostic, LabeledSpan, Severity, SourceCode};
use thiserror::Error;

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
    // fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn miette::Diagnostic> + 'a>> {
    //     if self.first {
    //         None
    //     } else {
    //         let nested_related: Vec<&dyn miette::Diagnostic> = self
    //             .sources
    //             .iter()
    //             .flat_map(|source| {
    //                 let mut related_diagnostics: Vec<&dyn miette::Diagnostic> = vec![source];
    //                 if let ShellError::EvalBlockWithInput { sources, .. } = source {
    //                     related_diagnostics
    //                         .extend(sources.iter().map(|s| s as &dyn miette::Diagnostic));
    //                 }
    //                 related_diagnostics
    //             })
    //             .collect();
    //         if nested_related.is_empty() {
    //             None
    //         } else {
    //             Some(Box::new(nested_related.into_iter()))
    //         }
    //     }
    // }
    fn code<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        if self.first {
            self.sources[0].code()
        } else {
            Some(Box::new("chainerr"))
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
