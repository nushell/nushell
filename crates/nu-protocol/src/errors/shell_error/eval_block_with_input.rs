use crate::{ShellError, Span};
use miette::{Diagnostic, LabeledSpan, Severity, SourceSpan};
use std::{
    error::Error as StdError,
    fmt::{self, Display},
};

/// An error raised while a closure ran on one pipeline input value.
///
/// Commands that run a closure per input value (`each`, `par-each`, `items`) wrap the closure's
/// error in this type so the report also points at the value being processed. With
/// `NU_BACKTRACE` set, an error from a stream is also wrapped once for each call the stream
/// passed through, labeled with [`CALLER`](Self::CALLER).
///
/// It reports as the wrapped error: the message, code, help, and related errors are the
/// wrapped error's, and its labels gain [`label`](Self::label) at [`span`](Self::span).
/// `try`/`catch` therefore sees the wrapped error's message in `$e.msg`.
///
/// A wrapped error that resolves its labels against its own source code can't share a snippet
/// with [`span`](Self::span). The report then shows the message with only this label, and
/// lists the wrapped error as related.
#[derive(Debug, Clone, PartialEq)]
pub struct EvalBlockWithInputError {
    /// Span of the input value the closure was running on, or of the call a failing stream
    /// passed through.
    pub span: Span,

    /// Text of the label at [`span`](Self::span), [`SOURCE_VALUE`](Self::SOURCE_VALUE) or
    /// [`CALLER`](Self::CALLER).
    pub label: &'static str,

    /// The error the closure raised.
    pub error: Box<ShellError>,
}

impl EvalBlockWithInputError {
    /// Label for the input value a closure was running on.
    pub const SOURCE_VALUE: &'static str = "source value";

    /// Label for a call that a failing stream passed through, the same text
    /// [`ChainedError`](crate::ChainedError) uses for a backtrace frame.
    pub const CALLER: &'static str = "error happened when running this";

    /// Wraps `error` with a `label` at `span`.
    ///
    /// `exit` and interrupts are returned unwrapped: they aren't about the input value, and the
    /// code that handles them (`catch`, `finally`, the REPL, `nu -c`) matches the bare variant.
    pub fn wrap(error: ShellError, span: Span, label: &'static str) -> ShellError {
        if error.is_interrupt() || matches!(error, ShellError::Exit { .. }) {
            return error;
        }
        ShellError::EvalBlockWithInput(Self {
            span,
            label,
            error: Box::new(error),
        })
    }

    /// The wrapped error when its labels can share a snippet with this one. An error with its
    /// own source code resolves its labels against that source, where [`span`](Self::span)
    /// would point at unrelated text.
    fn inline_error(&self) -> Option<&ShellError> {
        self.error.source_code().is_none().then_some(&*self.error)
    }
}

impl Display for EvalBlockWithInputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.error, f)
    }
}

// `source()` stays `None`: miette prints the source chain as extra lines, which would repeat
// the wrapped error that this type already reports as itself.
impl StdError for EvalBlockWithInputError {}

// `source_code()` stays `None`: an inline error has none, and otherwise this type's only label
// is resolved by the caller against the working set.
impl Diagnostic for EvalBlockWithInputError {
    fn code<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
        self.error.code()
    }

    fn severity(&self) -> Option<Severity> {
        self.error.severity()
    }

    fn help<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
        self.inline_error().and_then(|error| error.help())
    }

    fn url<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
        self.inline_error().and_then(|error| error.url())
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        let mut labels: Vec<_> = self
            .inline_error()
            .and_then(|error| error.labels())
            .into_iter()
            .flatten()
            .collect();
        // A span the wrapped error already labels keeps that label: a stream's first caller
        // is usually the failing call itself. Of nested closures, only the innermost input
        // value is labeled, since that is the value the failing closure ran on.
        let span = SourceSpan::from(self.span);
        let already_labeled = labels.iter().any(|label| {
            *label.inner() == span
                || (self.label == Self::SOURCE_VALUE && label.label() == Some(Self::SOURCE_VALUE))
        });
        if !already_labeled {
            labels.push(LabeledSpan::new_with_span(Some(self.label.into()), span));
        }
        Some(Box::new(labels.into_iter()))
    }

    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn Diagnostic> + 'a>> {
        match self.inline_error() {
            Some(error) => error.related(),
            None => Some(Box::new(std::iter::once(&*self.error as &dyn Diagnostic))),
        }
    }

    fn diagnostic_source(&self) -> Option<&dyn Diagnostic> {
        self.inline_error()
            .and_then(|error| error.diagnostic_source())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::shell_error::generic::GenericError;

    fn input_error(error: ShellError, span: Span) -> ShellError {
        EvalBlockWithInputError::wrap(error, span, EvalBlockWithInputError::SOURCE_VALUE)
    }

    fn label_texts(error: &dyn Diagnostic) -> Vec<Option<String>> {
        error
            .labels()
            .into_iter()
            .flatten()
            .map(|label| label.label().map(str::to_string))
            .collect()
    }

    #[test]
    fn reports_as_the_wrapped_error() {
        let inner = GenericError::new("boom", "here", Span::new(10, 12)).with_code("my::code");
        let error = input_error(inner.into(), Span::new(1, 2));

        assert_eq!(error.to_string(), "boom");
        assert_eq!(
            error.code().map(|code| code.to_string()),
            Some("my::code".into())
        );
        assert_eq!(
            label_texts(&error),
            [Some("here".into()), Some("source value".into())]
        );
    }

    #[test]
    fn reports_wrapped_error_with_own_source_as_related() {
        let inner = ShellError::from(GenericError::new_internal("boom", "here"));
        let error = input_error(inner.clone(), Span::new(1, 2));

        assert_eq!(error.to_string(), "boom");
        assert_eq!(label_texts(&error), [Some("source value".into())]);
        assert!(error.source_code().is_none());
        let related: Vec<_> = error.related().into_iter().flatten().collect();
        assert_eq!(related.len(), 1);
        assert_eq!(related[0].to_string(), "boom");
        assert_eq!(label_texts(related[0]), [Some("here".into())]);
    }

    #[test]
    fn labels_only_the_innermost_input_value() {
        let inner = GenericError::new("boom", "here", Span::new(10, 12));
        let error = input_error(input_error(inner.into(), Span::new(5, 6)), Span::new(1, 2));

        let spans: Vec<_> = error
            .labels()
            .into_iter()
            .flatten()
            .map(|label| *label.inner())
            .collect();
        assert_eq!(spans, [Span::new(10, 12).into(), Span::new(5, 6).into()]);
    }

    #[test]
    fn labels_each_caller_once() {
        let inner = GenericError::new("boom", "here", Span::new(10, 12));
        let caller = |error, span| {
            EvalBlockWithInputError::wrap(error, span, EvalBlockWithInputError::CALLER)
        };
        let error = caller(caller(inner.into(), Span::new(10, 12)), Span::new(1, 2));

        assert_eq!(
            label_texts(&error),
            [
                Some("here".into()),
                Some("error happened when running this".into())
            ]
        );
    }

    #[test]
    fn passes_control_flow_through() {
        let exit = ShellError::Exit {
            code: 4,
            abort: false,
        };
        assert_eq!(input_error(exit.clone(), Span::new(1, 2)), exit);

        let interrupted = ShellError::Interrupted {
            span: Span::new(3, 4),
        };
        assert_eq!(
            input_error(interrupted.clone(), Span::new(1, 2)),
            interrupted
        );
    }
}
