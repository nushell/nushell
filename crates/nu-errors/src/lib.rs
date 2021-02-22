use bigdecimal::BigDecimal;
use codespan_reporting::diagnostic::{Diagnostic, Label};
use derive_new::new;
use getset::Getters;
use nu_ansi_term::Color;
use nu_source::{
    DbgDocBldr, DebugDocBuilder, HasFallibleSpan, PrettyDebug, Span, Spanned, SpannedItem,
};
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Range;

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
#[derive(Debug, Clone, Getters, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub struct ParseError {
    #[get = "pub"]
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

/// ArgumentError describes various ways that the parser could fail because of unexpected arguments.
/// Nu commands are like a combination of functions and macros, and these errors correspond to
/// problems that could be identified during expansion based on the syntactic signature of a
/// command.
#[derive(Debug, Eq, PartialEq, Clone, Ord, Hash, PartialOrd, Serialize, Deserialize)]
pub enum ArgumentError {
    /// The command specified a mandatory flag, but it was missing.
    MissingMandatoryFlag(String),
    /// The command specified a mandatory positional argument, but it was missing.
    MissingMandatoryPositional(String),
    /// A flag was found, and it should have been followed by a value, but no value was found
    MissingValueForName(String),
    /// An argument was found, but the command does not recognize it
    UnexpectedArgument(Spanned<String>),
    /// An flag was found, but the command does not recognize it
    UnexpectedFlag(Spanned<String>),
    /// A sequence of characters was found that was not syntactically valid (but would have
    /// been valid if the command was an external command)
    InvalidExternalWord,
}

impl PrettyDebug for ArgumentError {
    fn pretty(&self) -> DebugDocBuilder {
        match self {
            ArgumentError::MissingMandatoryFlag(flag) => {
                DbgDocBldr::description("missing `")
                    + DbgDocBldr::description(flag)
                    + DbgDocBldr::description("` as mandatory flag")
            }
            ArgumentError::UnexpectedArgument(name) => {
                DbgDocBldr::description("unexpected `")
                    + DbgDocBldr::description(&name.item)
                    + DbgDocBldr::description("` is not supported")
            }
            ArgumentError::UnexpectedFlag(name) => {
                DbgDocBldr::description("unexpected `")
                    + DbgDocBldr::description(&name.item)
                    + DbgDocBldr::description("` is not supported")
            }
            ArgumentError::MissingMandatoryPositional(pos) => {
                DbgDocBldr::description("missing `")
                    + DbgDocBldr::description(pos)
                    + DbgDocBldr::description("` as mandatory positional argument")
            }
            ArgumentError::MissingValueForName(name) => {
                DbgDocBldr::description("missing value for flag `")
                    + DbgDocBldr::description(name)
                    + DbgDocBldr::description("`")
            }
            ArgumentError::InvalidExternalWord => DbgDocBldr::description("invalid word"),
        }
    }
}

/// A `ShellError` is a proximate error and a possible cause, which could have its own cause,
/// creating a cause chain.
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Clone, Serialize, Deserialize, Hash)]
pub struct ShellError {
    pub error: ProximateShellError,
    pub cause: Option<Box<ShellError>>,
}

/// `PrettyDebug` is for internal debugging. For user-facing debugging, [into_diagnostic](ShellError::into_diagnostic)
/// is used, which prints an error, highlighting spans.
impl PrettyDebug for ShellError {
    fn pretty(&self) -> DebugDocBuilder {
        match &self.error {
            ProximateShellError::SyntaxError { problem } => {
                DbgDocBldr::error("Syntax Error")
                    + DbgDocBldr::space()
                    + DbgDocBldr::delimit("(", DbgDocBldr::description(&problem.item), ")")
            }
            ProximateShellError::UnexpectedEof { .. } => DbgDocBldr::error("Unexpected end"),
            ProximateShellError::TypeError { expected, actual } => {
                DbgDocBldr::error("Type Error")
                    + DbgDocBldr::space()
                    + DbgDocBldr::delimit(
                        "(",
                        DbgDocBldr::description("expected:")
                            + DbgDocBldr::space()
                            + DbgDocBldr::description(expected)
                            + DbgDocBldr::description(",")
                            + DbgDocBldr::space()
                            + DbgDocBldr::description("actual:")
                            + DbgDocBldr::space()
                            + DbgDocBldr::option(actual.item.as_ref().map(DbgDocBldr::description)),
                        ")",
                    )
            }
            ProximateShellError::MissingProperty { subpath, expr } => {
                DbgDocBldr::error("Missing Property")
                    + DbgDocBldr::space()
                    + DbgDocBldr::delimit(
                        "(",
                        DbgDocBldr::description("expr:")
                            + DbgDocBldr::space()
                            + DbgDocBldr::description(&expr.item)
                            + DbgDocBldr::description(",")
                            + DbgDocBldr::space()
                            + DbgDocBldr::description("subpath:")
                            + DbgDocBldr::space()
                            + DbgDocBldr::description(&subpath.item),
                        ")",
                    )
            }
            ProximateShellError::InvalidIntegerIndex { subpath, .. } => {
                DbgDocBldr::error("Invalid integer index")
                    + DbgDocBldr::space()
                    + DbgDocBldr::delimit(
                        "(",
                        DbgDocBldr::description("subpath:")
                            + DbgDocBldr::space()
                            + DbgDocBldr::description(&subpath.item),
                        ")",
                    )
            }
            ProximateShellError::MissingValue { reason, .. } => {
                DbgDocBldr::error("Missing Value")
                    + DbgDocBldr::space()
                    + DbgDocBldr::delimit(
                        "(",
                        DbgDocBldr::description("reason:")
                            + DbgDocBldr::space()
                            + DbgDocBldr::description(reason),
                        ")",
                    )
            }
            ProximateShellError::ArgumentError { command, error } => {
                DbgDocBldr::error("Argument Error")
                    + DbgDocBldr::space()
                    + DbgDocBldr::delimit(
                        "(",
                        DbgDocBldr::description("command:")
                            + DbgDocBldr::space()
                            + DbgDocBldr::description(&command.item)
                            + DbgDocBldr::description(",")
                            + DbgDocBldr::space()
                            + DbgDocBldr::description("error:")
                            + DbgDocBldr::space()
                            + error.pretty(),
                        ")",
                    )
            }
            ProximateShellError::RangeError {
                kind,
                actual_kind,
                operation,
            } => {
                DbgDocBldr::error("Range Error")
                    + DbgDocBldr::space()
                    + DbgDocBldr::delimit(
                        "(",
                        DbgDocBldr::description("expected:")
                            + DbgDocBldr::space()
                            + kind.pretty()
                            + DbgDocBldr::description(",")
                            + DbgDocBldr::space()
                            + DbgDocBldr::description("actual:")
                            + DbgDocBldr::space()
                            + DbgDocBldr::description(&actual_kind.item)
                            + DbgDocBldr::description(",")
                            + DbgDocBldr::space()
                            + DbgDocBldr::description("operation:")
                            + DbgDocBldr::space()
                            + DbgDocBldr::description(operation),
                        ")",
                    )
            }
            ProximateShellError::Diagnostic(_) => DbgDocBldr::error("diagnostic"),
            ProximateShellError::CoerceError { left, right } => {
                DbgDocBldr::error("Coercion Error")
                    + DbgDocBldr::space()
                    + DbgDocBldr::delimit(
                        "(",
                        DbgDocBldr::description("left:")
                            + DbgDocBldr::space()
                            + DbgDocBldr::description(&left.item)
                            + DbgDocBldr::description(",")
                            + DbgDocBldr::space()
                            + DbgDocBldr::description("right:")
                            + DbgDocBldr::space()
                            + DbgDocBldr::description(&right.item),
                        ")",
                    )
            }
            ProximateShellError::UntaggedRuntimeError { reason } => {
                DbgDocBldr::error("Unknown Error")
                    + DbgDocBldr::delimit("(", DbgDocBldr::description(reason), ")")
            }
            ProximateShellError::ExternalPlaceholderError => {
                DbgDocBldr::error("non-zero external exit code")
            }
        }
    }
}

impl std::fmt::Display for ShellError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.pretty().display())
    }
}

impl serde::de::Error for ShellError {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        ShellError::untagged_runtime_error(msg.to_string())
    }
}

impl ShellError {
    /// An error that describes a mismatch between the given type and the expected type
    pub fn type_error(
        expected: impl Into<String>,
        actual: Spanned<impl Into<String>>,
    ) -> ShellError {
        ProximateShellError::TypeError {
            expected: expected.into(),
            actual: actual.map(|i| Some(i.into())),
        }
        .start()
    }

    pub fn missing_property(
        subpath: Spanned<impl Into<String>>,
        expr: Spanned<impl Into<String>>,
    ) -> ShellError {
        ProximateShellError::MissingProperty {
            subpath: subpath.map(|s| s.into()),
            expr: expr.map(|e| e.into()),
        }
        .start()
    }

    pub fn missing_value(span: impl Into<Option<Span>>, reason: impl Into<String>) -> ShellError {
        ProximateShellError::MissingValue {
            span: span.into(),
            reason: reason.into(),
        }
        .start()
    }

    pub fn invalid_integer_index(
        subpath: Spanned<impl Into<String>>,
        integer: impl Into<Span>,
    ) -> ShellError {
        ProximateShellError::InvalidIntegerIndex {
            subpath: subpath.map(|s| s.into()),
            integer: integer.into(),
        }
        .start()
    }

    pub fn untagged_runtime_error(error: impl Into<String>) -> ShellError {
        ProximateShellError::UntaggedRuntimeError {
            reason: error.into(),
        }
        .start()
    }

    pub fn unexpected_eof(expected: impl Into<String>, span: impl Into<Span>) -> ShellError {
        ProximateShellError::UnexpectedEof {
            expected: expected.into(),
            span: span.into(),
        }
        .start()
    }

    pub fn range_error(
        expected: impl Into<ExpectedRange>,
        actual: &Spanned<impl fmt::Debug>,
        operation: impl Into<String>,
    ) -> ShellError {
        ProximateShellError::RangeError {
            kind: expected.into(),
            actual_kind: format!("{:?}", actual.item).spanned(actual.span),
            operation: operation.into(),
        }
        .start()
    }

    pub fn syntax_error(problem: Spanned<impl Into<String>>) -> ShellError {
        ProximateShellError::SyntaxError {
            problem: problem.map(|p| p.into()),
        }
        .start()
    }

    pub fn coerce_error(
        left: Spanned<impl Into<String>>,
        right: Spanned<impl Into<String>>,
    ) -> ShellError {
        ProximateShellError::CoerceError {
            left: left.map(|l| l.into()),
            right: right.map(|r| r.into()),
        }
        .start()
    }

    pub fn argument_error(command: Spanned<impl Into<String>>, kind: ArgumentError) -> ShellError {
        ProximateShellError::ArgumentError {
            command: command.map(|c| c.into()),
            error: kind,
        }
        .start()
    }

    pub fn diagnostic(diagnostic: Diagnostic<usize>) -> ShellError {
        ProximateShellError::Diagnostic(ShellDiagnostic { diagnostic }).start()
    }

    pub fn external_non_zero() -> ShellError {
        ProximateShellError::ExternalPlaceholderError.start()
    }

    pub fn into_diagnostic(self) -> Option<Diagnostic<usize>> {
        match self.error {
            ProximateShellError::MissingValue { span, reason } => {
                let mut d = Diagnostic::bug().with_message(format!("Internal Error (missing value) :: {}", reason));

                if let Some(span) = span {
                    d = d.with_labels(vec![Label::primary(0, span)]);
                }

                Some(d)
            }
            ProximateShellError::ArgumentError {
                command,
                error,
            } => Some(match error {
                ArgumentError::InvalidExternalWord => Diagnostic::error().with_message("Invalid bare word for Nu command (did you intend to invoke an external command?)")
                .with_labels(vec![Label::primary(0, command.span)]),
                ArgumentError::UnexpectedArgument(argument) => Diagnostic::error().with_message(
                    format!(
                        "{} unexpected {}",
                        Color::Cyan.paint(&command.item),
                        Color::Green.bold().paint(&argument.item)
                    )
                )
                .with_labels(
                    vec![Label::primary(0, argument.span).with_message(
                        format!("unexpected argument (try {} -h)", &command.item))]
                ),
                ArgumentError::UnexpectedFlag(flag) => Diagnostic::error().with_message(
                    format!(
                        "{} unexpected {}",
                        Color::Cyan.paint(&command.item),
                        Color::Green.bold().paint(&flag.item)
                    ),
                )
                .with_labels(vec![
                    Label::primary(0, flag.span).with_message(
                    format!("unexpected flag (try {} -h)", &command.item))
                    ]),
                ArgumentError::MissingMandatoryFlag(name) => Diagnostic::error().with_message(                    format!(
                        "{} requires {}{}",
                        Color::Cyan.paint(&command.item),
                        Color::Green.bold().paint("--"),
                        Color::Green.bold().paint(name)
                    ),
                )
                .with_labels(vec![Label::primary(0, command.span)]),
                ArgumentError::MissingMandatoryPositional(name) => Diagnostic::error().with_message(
                    format!(
                        "{} requires {} parameter",
                        Color::Cyan.paint(&command.item),
                        Color::Green.bold().paint(name.clone())
                    ),
                )
                .with_labels(
                    vec![Label::primary(0, command.span).with_message(format!("requires {} parameter", name))],
                ),
                ArgumentError::MissingValueForName(name) => Diagnostic::error().with_message(
                    format!(
                        "{} is missing value for flag {}{}",
                        Color::Cyan.paint(&command.item),
                        Color::Green.bold().paint("--"),
                        Color::Green.bold().paint(name)
                    ),
                )
                .with_labels(vec![Label::primary(0, command.span)]),
            }),
            ProximateShellError::TypeError {
                expected,
                actual:
                    Spanned {
                        item: Some(actual),
                        span,
                    },
            } => Some(Diagnostic::error().with_message("Type Error").with_labels(
                vec![Label::primary(0, span)
                    .with_message(format!("Expected {}, found {}", expected, actual))]),
            ),
            ProximateShellError::TypeError {
                expected,
                actual:
                    Spanned {
                        item: None,
                        span
                    },
            } => Some(Diagnostic::error().with_message("Type Error")
                .with_labels(vec![Label::primary(0, span).with_message(expected)])),

            ProximateShellError::UnexpectedEof {
                expected, span
            } => Some(Diagnostic::error().with_message("Unexpected end of input")
                .with_labels(vec![Label::primary(0, span).with_message(format!("Expected {}", expected))])),

            ProximateShellError::RangeError {
                kind,
                operation,
                actual_kind:
                    Spanned {
                        item,
                        span
                    },
            } => Some(Diagnostic::error().with_message("Range Error").with_labels(
                vec![Label::primary(0, span).with_message(format!(
                    "Expected to convert {} to {} while {}, but it was out of range",
                    item,
                    kind.display(),
                    operation
                ))]),
            ),

            ProximateShellError::SyntaxError {
                problem:
                    Spanned {
                        span,
                        item
                    },
            } => Some(Diagnostic::error().with_message("Syntax Error")
                .with_labels(vec![Label::primary(0, span).with_message(item)])),

            ProximateShellError::MissingProperty { subpath, expr, .. } => {

                let mut diag = Diagnostic::error().with_message("Missing property");

                if subpath.span == Span::unknown() {
                    diag.message = format!("Missing property (for {})", subpath.item);
                } else {
                    let subpath = Label::primary(0, subpath.span).with_message(subpath.item);
                    let mut labels = vec![subpath];

                    if expr.span != Span::unknown() {
                        let expr = Label::primary(0, expr.span).with_message(expr.item);
                        labels.push(expr);
                    }
                    diag = diag.with_labels(labels);
                }

                Some(diag)
            }

            ProximateShellError::InvalidIntegerIndex { subpath,integer } => {
                let mut diag = Diagnostic::error().with_message("Invalid integer property");
                let mut labels = vec![];
                if subpath.span == Span::unknown() {
                    diag.message = format!("Invalid integer property (for {})", subpath.item)
                } else {
                    let label = Label::primary(0, subpath.span).with_message(subpath.item);
                    labels.push(label);
                }

                labels.push(Label::secondary(0, integer).with_message("integer"));
                diag = diag.with_labels(labels);

                Some(diag)
            }

            ProximateShellError::Diagnostic(diag) => Some(diag.diagnostic),
            ProximateShellError::CoerceError { left, right } => {
                Some(Diagnostic::error().with_message("Coercion error")
                    .with_labels(vec![Label::primary(0, left.span).with_message(left.item),
                    Label::secondary(0, right.span).with_message(right.item)]))
            }

            ProximateShellError::UntaggedRuntimeError { reason } => Some(Diagnostic::error().with_message(format!("Error: {}", reason))),
            ProximateShellError::ExternalPlaceholderError => None,
        }
    }

    pub fn labeled_error(
        msg: impl Into<String>,
        label: impl Into<String>,
        span: impl Into<Span>,
    ) -> ShellError {
        ShellError::diagnostic(
            Diagnostic::error()
                .with_message(msg.into())
                .with_labels(vec![
                    Label::primary(0, span.into()).with_message(label.into())
                ]),
        )
    }

    pub fn labeled_error_with_secondary(
        msg: impl Into<String>,
        primary_label: impl Into<String>,
        primary_span: impl Into<Span>,
        secondary_label: impl Into<String>,
        secondary_span: impl Into<Span>,
    ) -> ShellError {
        ShellError::diagnostic(
            Diagnostic::error()
                .with_message(msg.into())
                .with_labels(vec![
                    Label::primary(0, primary_span.into()).with_message(primary_label.into()),
                    Label::secondary(0, secondary_span.into()).with_message(secondary_label.into()),
                ]),
        )
    }

    pub fn unimplemented(title: impl Into<String>) -> ShellError {
        ShellError::untagged_runtime_error(&format!("Unimplemented: {}", title.into()))
    }

    pub fn unexpected(title: impl Into<String>) -> ShellError {
        ShellError::untagged_runtime_error(&format!("Unexpected: {}", title.into()))
    }
}

/// `ExpectedRange` describes a range of values that was expected by a command. In addition
/// to typical ranges, this enum allows an error to specify that the range of allowed values
/// corresponds to a particular numeric type (which is a dominant use-case for the
/// [RangeError](ProximateShellError::RangeError) error type).
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Clone, Serialize, Deserialize)]
pub enum ExpectedRange {
    I8,
    I16,
    I32,
    I64,
    I128,
    U8,
    U16,
    U32,
    U64,
    U128,
    F32,
    F64,
    Usize,
    Size,
    BigInt,
    BigDecimal,
    Range { start: usize, end: usize },
}

/// Convert a Rust range into an [ExpectedRange](ExpectedRange).
impl From<Range<usize>> for ExpectedRange {
    fn from(range: Range<usize>) -> Self {
        ExpectedRange::Range {
            start: range.start,
            end: range.end,
        }
    }
}

impl PrettyDebug for ExpectedRange {
    fn pretty(&self) -> DebugDocBuilder {
        DbgDocBldr::description(match self {
            ExpectedRange::I8 => "an 8-bit signed integer",
            ExpectedRange::I16 => "a 16-bit signed integer",
            ExpectedRange::I32 => "a 32-bit signed integer",
            ExpectedRange::I64 => "a 64-bit signed integer",
            ExpectedRange::I128 => "a 128-bit signed integer",
            ExpectedRange::U8 => "an 8-bit unsigned integer",
            ExpectedRange::U16 => "a 16-bit unsigned integer",
            ExpectedRange::U32 => "a 32-bit unsigned integer",
            ExpectedRange::U64 => "a 64-bit unsigned integer",
            ExpectedRange::U128 => "a 128-bit unsigned integer",
            ExpectedRange::F32 => "a 32-bit float",
            ExpectedRange::F64 => "a 64-bit float",
            ExpectedRange::Usize => "an list index",
            ExpectedRange::Size => "a list offset",
            ExpectedRange::BigDecimal => "a decimal",
            ExpectedRange::BigInt => "an integer",
            ExpectedRange::Range { start, end } => {
                return DbgDocBldr::description(format!("{} to {}", start, end))
            }
        })
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Ord, PartialOrd, Serialize, Deserialize, Hash)]
pub enum ProximateShellError {
    SyntaxError {
        problem: Spanned<String>,
    },
    UnexpectedEof {
        expected: String,
        span: Span,
    },
    TypeError {
        expected: String,
        actual: Spanned<Option<String>>,
    },
    MissingProperty {
        subpath: Spanned<String>,
        expr: Spanned<String>,
    },
    InvalidIntegerIndex {
        subpath: Spanned<String>,
        integer: Span,
    },
    MissingValue {
        span: Option<Span>,
        reason: String,
    },
    ArgumentError {
        command: Spanned<String>,
        error: ArgumentError,
    },
    RangeError {
        kind: ExpectedRange,
        actual_kind: Spanned<String>,
        operation: String,
    },
    Diagnostic(ShellDiagnostic),
    CoerceError {
        left: Spanned<String>,
        right: Spanned<String>,
    },
    UntaggedRuntimeError {
        reason: String,
    },
    ExternalPlaceholderError,
}

impl ProximateShellError {
    fn start(self) -> ShellError {
        ShellError {
            cause: None,
            error: self,
        }
    }
}

impl HasFallibleSpan for ShellError {
    fn maybe_span(&self) -> Option<Span> {
        self.error.maybe_span()
    }
}

impl HasFallibleSpan for ProximateShellError {
    fn maybe_span(&self) -> Option<Span> {
        Some(match self {
            ProximateShellError::SyntaxError { problem } => problem.span,
            ProximateShellError::UnexpectedEof { span, .. } => *span,
            ProximateShellError::TypeError { actual, .. } => actual.span,
            ProximateShellError::MissingProperty { subpath, .. } => subpath.span,
            ProximateShellError::InvalidIntegerIndex { subpath, .. } => subpath.span,
            ProximateShellError::MissingValue { span, .. } => return *span,
            ProximateShellError::ArgumentError { command, .. } => command.span,
            ProximateShellError::RangeError { actual_kind, .. } => actual_kind.span,
            ProximateShellError::Diagnostic(_) => return None,
            ProximateShellError::CoerceError { left, right } => left.span.until(right.span),
            ProximateShellError::UntaggedRuntimeError { .. } => return None,
            ProximateShellError::ExternalPlaceholderError => return None,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellDiagnostic {
    pub diagnostic: Diagnostic<usize>,
}

impl std::hash::Hash for ShellDiagnostic {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.diagnostic.severity.hash(state);
        self.diagnostic.code.hash(state);
        self.diagnostic.message.hash(state);

        for label in &self.diagnostic.labels {
            label.range.hash(state);
            label.message.hash(state);
            match label.style {
                codespan_reporting::diagnostic::LabelStyle::Primary => 0.hash(state),
                codespan_reporting::diagnostic::LabelStyle::Secondary => 1.hash(state),
            }
        }
    }
}

impl PartialEq for ShellDiagnostic {
    fn eq(&self, _other: &ShellDiagnostic) -> bool {
        false
    }
}

impl Eq for ShellDiagnostic {}

impl std::cmp::PartialOrd for ShellDiagnostic {
    fn partial_cmp(&self, _other: &Self) -> Option<std::cmp::Ordering> {
        Some(std::cmp::Ordering::Less)
    }
}

impl std::cmp::Ord for ShellDiagnostic {
    fn cmp(&self, _other: &Self) -> std::cmp::Ordering {
        std::cmp::Ordering::Less
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, new, Clone, Serialize, Deserialize)]
pub struct StringError {
    title: String,
    error: String,
}

impl std::error::Error for ShellError {}

impl std::convert::From<Box<dyn std::error::Error>> for ShellError {
    fn from(input: Box<dyn std::error::Error>) -> ShellError {
        ShellError::untagged_runtime_error(format!("{}", input))
    }
}

impl std::convert::From<std::io::Error> for ShellError {
    fn from(input: std::io::Error) -> ShellError {
        ShellError::untagged_runtime_error(format!("{}", input))
    }
}

impl std::convert::From<std::string::FromUtf8Error> for ShellError {
    fn from(input: std::string::FromUtf8Error) -> ShellError {
        ShellError::untagged_runtime_error(format!("{}", input))
    }
}

impl std::convert::From<std::str::Utf8Error> for ShellError {
    fn from(input: std::str::Utf8Error) -> ShellError {
        ShellError::untagged_runtime_error(format!("{}", input))
    }
}

impl std::convert::From<serde_yaml::Error> for ShellError {
    fn from(input: serde_yaml::Error) -> ShellError {
        ShellError::untagged_runtime_error(format!("{:?}", input))
    }
}

impl std::convert::From<toml::ser::Error> for ShellError {
    fn from(input: toml::ser::Error) -> ShellError {
        ShellError::untagged_runtime_error(format!("{:?}", input))
    }
}

impl std::convert::From<serde_json::Error> for ShellError {
    fn from(input: serde_json::Error) -> ShellError {
        ShellError::untagged_runtime_error(format!("{:?}", input))
    }
}

impl std::convert::From<Box<dyn std::error::Error + Send + Sync>> for ShellError {
    fn from(input: Box<dyn std::error::Error + Send + Sync>) -> ShellError {
        ShellError::untagged_runtime_error(format!("{:?}", input))
    }
}

impl std::convert::From<glob::PatternError> for ShellError {
    fn from(input: glob::PatternError) -> ShellError {
        ShellError::untagged_runtime_error(format!("{:?}", input))
    }
}

pub trait CoerceInto<U> {
    fn coerce_into(self, operation: impl Into<String>) -> Result<U, ShellError>;
}

trait ToExpectedRange {
    fn to_expected_range() -> ExpectedRange;
}

macro_rules! ranged_int {
    ($ty:tt -> $op:tt -> $variant:tt) => {
        impl ToExpectedRange for $ty {
            fn to_expected_range() -> ExpectedRange {
                ExpectedRange::$variant
            }
        }

        impl CoerceInto<$ty> for nu_source::Tagged<BigInt> {
            fn coerce_into(self, operation: impl Into<String>) -> Result<$ty, ShellError> {
                self.$op().ok_or_else(|| {
                    ShellError::range_error(
                        $ty::to_expected_range(),
                        &self.item.spanned(self.tag.span),
                        operation.into(),
                    )
                })
            }
        }

        impl CoerceInto<$ty> for nu_source::Tagged<&BigInt> {
            fn coerce_into(self, operation: impl Into<String>) -> Result<$ty, ShellError> {
                self.$op().ok_or_else(|| {
                    ShellError::range_error(
                        $ty::to_expected_range(),
                        &self.item.spanned(self.tag.span),
                        operation.into(),
                    )
                })
            }
        }
    };
}

ranged_int!(u8  -> to_u8  -> U8);
ranged_int!(u16 -> to_u16 -> U16);
ranged_int!(u32 -> to_u32 -> U32);
ranged_int!(u64 -> to_u64 -> U64);
ranged_int!(i8  -> to_i8  -> I8);
ranged_int!(i16 -> to_i16 -> I16);
ranged_int!(i32 -> to_i32 -> I32);
ranged_int!(i64 -> to_i64 -> I64);

macro_rules! ranged_decimal {
    ($ty:tt -> $op:tt -> $variant:tt) => {
        impl ToExpectedRange for $ty {
            fn to_expected_range() -> ExpectedRange {
                ExpectedRange::$variant
            }
        }

        impl CoerceInto<$ty> for nu_source::Tagged<BigDecimal> {
            fn coerce_into(self, operation: impl Into<String>) -> Result<$ty, ShellError> {
                self.$op().ok_or_else(|| {
                    ShellError::range_error(
                        $ty::to_expected_range(),
                        &self.item.spanned(self.tag.span),
                        operation.into(),
                    )
                })
            }
        }

        impl CoerceInto<$ty> for nu_source::Tagged<&BigDecimal> {
            fn coerce_into(self, operation: impl Into<String>) -> Result<$ty, ShellError> {
                self.$op().ok_or_else(|| {
                    ShellError::range_error(
                        $ty::to_expected_range(),
                        &self.item.spanned(self.tag.span),
                        operation.into(),
                    )
                })
            }
        }
    };
}

ranged_decimal!(f32 -> to_f32 -> F32);
ranged_decimal!(f64 -> to_f64 -> F64);
