use nu_errors::{ArgumentError, ShellError};
use nu_source::{Span, Spanned, SpannedItem};
use serde::{Deserialize, Serialize};

/// A structured reason for a ParseError. Note that parsing in nu is more like macro expansion in
/// other languages, so the kinds of errors that can occur during parsing are more contextual than
/// you might expect.
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum ParseErrorReason {
    /// The parser encountered an EOF rather than what it was expecting
    Eof { expected: String, span: Span },
    /// The parser expected to see the end of a token stream (possibly the token
    /// stream from inside a delimited token node), but found something else.
    ExtraTokens { actual: Spanned<String> },
    /// The parser encountered something other than what it was expecting
    Mismatch {
        expected: String,
        actual: Spanned<String>,
    },

    /// An unexpected internal error has occurred
    InternalError { message: Spanned<String> },

    /// The parser tried to parse an argument for a command, but it failed for
    /// some reason
    ArgumentError {
        command: Spanned<String>,
        error: ArgumentError,
    },
}

/// A newtype for `ParseErrorReason`
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub struct ParseError {
    reason: ParseErrorReason,
}

impl ParseError {
    /// Construct a [ParseErrorReason::Eof](ParseErrorReason::Eof)
    pub fn unexpected_eof(expected: impl Into<String>, span: Span) -> ParseError {
        ParseError {
            reason: ParseErrorReason::Eof {
                expected: expected.into(),
                span,
            },
        }
    }

    /// Construct a [ParseErrorReason::ExtraTokens](ParseErrorReason::ExtraTokens)
    pub fn extra_tokens(actual: Spanned<impl Into<String>>) -> ParseError {
        let Spanned { span, item } = actual;

        ParseError {
            reason: ParseErrorReason::ExtraTokens {
                actual: item.into().spanned(span),
            },
        }
    }

    /// Construct a [ParseErrorReason::Mismatch](ParseErrorReason::Mismatch)
    pub fn mismatch(expected: impl Into<String>, actual: Spanned<impl Into<String>>) -> ParseError {
        let Spanned { span, item } = actual;

        ParseError {
            reason: ParseErrorReason::Mismatch {
                expected: expected.into(),
                actual: item.into().spanned(span),
            },
        }
    }

    /// Construct a [ParseErrorReason::InternalError](ParseErrorReason::InternalError)
    pub fn internal_error(message: Spanned<impl Into<String>>) -> ParseError {
        ParseError {
            reason: ParseErrorReason::InternalError {
                message: message.item.into().spanned(message.span),
            },
        }
    }

    /// Construct a [ParseErrorReason::ArgumentError](ParseErrorReason::ArgumentError)
    pub fn argument_error(command: Spanned<impl Into<String>>, kind: ArgumentError) -> ParseError {
        ParseError {
            reason: ParseErrorReason::ArgumentError {
                command: command.item.into().spanned(command.span),
                error: kind,
            },
        }
    }
}

/// Convert a [ParseError](ParseError) into a [ShellError](ShellError)
impl From<ParseError> for ShellError {
    fn from(error: ParseError) -> ShellError {
        match error.reason {
            ParseErrorReason::Eof { expected, span } => ShellError::unexpected_eof(expected, span),
            ParseErrorReason::ExtraTokens { actual } => ShellError::type_error("nothing", actual),
            ParseErrorReason::Mismatch { actual, expected } => {
                ShellError::type_error(expected, actual)
            }
            ParseErrorReason::InternalError { message } => ShellError::labeled_error(
                format!("Internal error: {}", message.item),
                &message.item,
                &message.span,
            ),
            ParseErrorReason::ArgumentError { command, error } => {
                ShellError::argument_error(command, error)
            }
        }
    }
}
