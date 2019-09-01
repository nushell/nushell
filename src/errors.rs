#[allow(unused)]
use crate::prelude::*;

use ansi_term::Color;
use derive_new::new;
use language_reporting::{Diagnostic, Label, Severity};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Eq, PartialEq, Clone, Ord, PartialOrd, Serialize, Deserialize)]
pub enum Description {
    Source(Tagged<String>),
    Synthetic(String),
}

impl Description {
    pub fn from(value: Tagged<impl Into<String>>) -> Description {
        let value_span = value.span();
        let value_tag = value.tag();

        match value_span {
            Span { start: 0, end: 0 } => Description::Synthetic(value.item.into()),
            _ => Description::Source(Tagged::from_item(value.item.into(), value_tag)),
        }
    }
}

impl Description {
    fn into_label(self) -> Result<Label<Span>, String> {
        match self {
            Description::Source(s) => Ok(Label::new_primary(s.span()).with_message(s.item)),
            Description::Synthetic(s) => Err(s),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Ord, PartialOrd, Serialize, Deserialize)]
pub enum ArgumentError {
    MissingMandatoryFlag(String),
    MissingMandatoryPositional(String),
    MissingValueForName(String),
}

// pub fn labelled(
//     span: impl Into<Span>,
//     heading: &'a str,
//     span_message: &'a str,
// ) -> impl FnOnce(ShellError) -> ShellError + 'a {
//     let span = span.into();

//     move |_| ShellError::labeled_error(heading, span_message, span)
// }

#[derive(Debug, Eq, PartialEq, Clone, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ShellError {
    error: ProximateShellError,
    cause: Option<Box<ProximateShellError>>,
}

impl ToDebug for ShellError {
    fn fmt_debug(&self, f: &mut fmt::Formatter, source: &str) -> fmt::Result {
        self.error.fmt_debug(f, source)
    }
}

impl serde::de::Error for ShellError {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        ShellError::string(msg.to_string())
    }
}

impl ShellError {
    pub(crate) fn type_error(
        expected: impl Into<String>,
        actual: Tagged<impl Into<String>>,
    ) -> ShellError {
        ProximateShellError::TypeError {
            expected: expected.into(),
            actual: actual.map(|i| Some(i.into())),
        }
        .start()
    }

    pub(crate) fn range_error(
        expected: impl Into<String>,
        actual: Tagged<impl fmt::Debug>,
    ) -> ShellError {
        ProximateShellError::RangeError {
            kind: expected.into(),
            actual_kind: actual.map(|a| format!("{:?}", a)),
        }
        .start()
    }

    pub(crate) fn syntax_error(problem: Tagged<impl Into<String>>) -> ShellError {
        ProximateShellError::SyntaxError {
            problem: problem.map(|p| p.into()),
        }
        .start()
    }

    pub(crate) fn invalid_command(problem: impl Into<Tag>) -> ShellError {
        ProximateShellError::InvalidCommand {
            command: problem.into(),
        }
        .start()
    }

    pub(crate) fn coerce_error(
        left: Tagged<impl Into<String>>,
        right: Tagged<impl Into<String>>,
    ) -> ShellError {
        ProximateShellError::CoerceError {
            left: left.map(|l| l.into()),
            right: right.map(|r| r.into()),
        }
        .start()
    }

    pub(crate) fn missing_property(subpath: Description, expr: Description) -> ShellError {
        ProximateShellError::MissingProperty { subpath, expr }.start()
    }

    pub(crate) fn missing_value(span: Option<Span>, reason: impl Into<String>) -> ShellError {
        ProximateShellError::MissingValue {
            span,
            reason: reason.into(),
        }
        .start()
    }

    pub(crate) fn argument_error(
        command: impl Into<String>,
        kind: ArgumentError,
        span: Span,
    ) -> ShellError {
        ProximateShellError::ArgumentError {
            command: command.into(),
            error: kind,
            span,
        }
        .start()
    }

    pub(crate) fn parse_error(
        error: nom::Err<(nom5_locate::LocatedSpan<&str>, nom::error::ErrorKind)>,
    ) -> ShellError {
        use language_reporting::*;

        match error {
            nom::Err::Incomplete(_) => {
                // TODO: Get span of EOF
                let diagnostic = Diagnostic::new(
                    Severity::Error,
                    format!("Parse Error: Unexpected end of line"),
                );

                ShellError::diagnostic(diagnostic)
            }
            nom::Err::Failure(span) | nom::Err::Error(span) => {
                let diagnostic = Diagnostic::new(Severity::Error, format!("Parse Error"))
                    .with_label(Label::new_primary(Span::from(span.0)));

                ShellError::diagnostic(diagnostic)
            }
        }
    }

    pub(crate) fn diagnostic(diagnostic: Diagnostic<Span>) -> ShellError {
        ProximateShellError::Diagnostic(ShellDiagnostic { diagnostic }).start()
    }

    pub(crate) fn to_diagnostic(self) -> Diagnostic<Span> {
        match self.error {
            ProximateShellError::String(StringError { title, .. }) => {
                Diagnostic::new(Severity::Error, title)
            }
            ProximateShellError::InvalidCommand { command } => {
                Diagnostic::new(Severity::Error, "Invalid command")
                    .with_label(Label::new_primary(command.span))
            }
            ProximateShellError::MissingValue { span, reason } => {
                let mut d = Diagnostic::new(
                    Severity::Bug,
                    format!("Internal Error (missing value) :: {}", reason),
                );

                if let Some(span) = span {
                    d = d.with_label(Label::new_primary(span));
                }

                d
            }
            ProximateShellError::ArgumentError {
                command,
                error,
                span,
            } => match error {
                ArgumentError::MissingMandatoryFlag(name) => Diagnostic::new(
                    Severity::Error,
                    format!(
                        "{} requires {}{}",
                        Color::Cyan.paint(command),
                        Color::Black.bold().paint("--"),
                        Color::Black.bold().paint(name)
                    ),
                )
                .with_label(Label::new_primary(span)),
                ArgumentError::MissingMandatoryPositional(name) => Diagnostic::new(
                    Severity::Error,
                    format!(
                        "{} requires {} parameter",
                        Color::Cyan.paint(command),
                        Color::Green.bold().paint(name.clone())
                    ),
                )
                .with_label(
                    Label::new_primary(span).with_message(format!("requires {} parameter", name)),
                ),
                ArgumentError::MissingValueForName(name) => Diagnostic::new(
                    Severity::Error,
                    format!(
                        "{} is missing value for flag {}{}",
                        Color::Cyan.paint(command),
                        Color::Black.bold().paint("--"),
                        Color::Black.bold().paint(name)
                    ),
                )
                .with_label(Label::new_primary(span)),
            },
            ProximateShellError::TypeError {
                expected,
                actual:
                    Tagged {
                        item: Some(actual),
                        tag: Tag { span, .. },
                    },
            } => Diagnostic::new(Severity::Error, "Type Error").with_label(
                Label::new_primary(span)
                    .with_message(format!("Expected {}, found {}", expected, actual)),
            ),

            ProximateShellError::TypeError {
                expected,
                actual:
                    Tagged {
                        item: None,
                        tag: Tag { span, .. },
                    },
            } => Diagnostic::new(Severity::Error, "Type Error")
                .with_label(Label::new_primary(span).with_message(expected)),

            ProximateShellError::RangeError {
                kind,
                actual_kind:
                    Tagged {
                        item,
                        tag: Tag { span, .. },
                    },
            } => Diagnostic::new(Severity::Error, "Range Error").with_label(
                Label::new_primary(span).with_message(format!(
                    "Expected to covert {} to {}, but it was out of range",
                    item, kind
                )),
            ),

            ProximateShellError::SyntaxError {
                problem:
                    Tagged {
                        tag: Tag { span, .. },
                        ..
                    },
            } => Diagnostic::new(Severity::Error, "Syntax Error")
                .with_label(Label::new_primary(span).with_message("Unexpected external command")),

            ProximateShellError::MissingProperty { subpath, expr } => {
                let subpath = subpath.into_label();
                let expr = expr.into_label();

                let mut diag = Diagnostic::new(Severity::Error, "Missing property");

                match subpath {
                    Ok(label) => diag = diag.with_label(label),
                    Err(ty) => diag.message = format!("Missing property (for {})", ty),
                }

                if let Ok(label) = expr {
                    diag = diag.with_label(label);
                }

                diag
            }

            ProximateShellError::Diagnostic(diag) => diag.diagnostic,
            ProximateShellError::CoerceError { left, right } => {
                Diagnostic::new(Severity::Error, "Coercion error")
                    .with_label(Label::new_primary(left.span()).with_message(left.item))
                    .with_label(Label::new_secondary(right.span()).with_message(right.item))
            }
        }
    }

    pub fn labeled_error(
        msg: impl Into<String>,
        label: impl Into<String>,
        span: impl Into<Span>,
    ) -> ShellError {
        ShellError::diagnostic(
            Diagnostic::new(Severity::Error, msg.into())
                .with_label(Label::new_primary(span.into()).with_message(label.into())),
        )
    }

    pub fn labeled_error_with_secondary(
        msg: impl Into<String>,
        primary_label: impl Into<String>,
        primary_span: Span,
        secondary_label: impl Into<String>,
        secondary_span: Span,
    ) -> ShellError {
        ShellError::diagnostic(
            Diagnostic::new_error(msg.into())
                .with_label(Label::new_primary(primary_span).with_message(primary_label.into()))
                .with_label(
                    Label::new_secondary(secondary_span).with_message(secondary_label.into()),
                ),
        )
    }

    pub fn string(title: impl Into<String>) -> ShellError {
        ProximateShellError::String(StringError::new(title.into(), Value::nothing())).start()
    }

    pub(crate) fn unimplemented(title: impl Into<String>) -> ShellError {
        ShellError::string(&format!("Unimplemented: {}", title.into()))
    }

    pub(crate) fn unexpected(title: impl Into<String>) -> ShellError {
        ShellError::string(&format!("Unexpected: {}", title.into()))
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Ord, PartialOrd, Serialize, Deserialize)]
pub enum ProximateShellError {
    String(StringError),
    SyntaxError {
        problem: Tagged<String>,
    },
    InvalidCommand {
        command: Tag,
    },
    TypeError {
        expected: String,
        actual: Tagged<Option<String>>,
    },
    MissingProperty {
        subpath: Description,
        expr: Description,
    },
    MissingValue {
        span: Option<Span>,
        reason: String,
    },
    ArgumentError {
        command: String,
        error: ArgumentError,
        span: Span,
    },
    RangeError {
        kind: String,
        actual_kind: Tagged<String>,
    },
    Diagnostic(ShellDiagnostic),
    CoerceError {
        left: Tagged<String>,
        right: Tagged<String>,
    },
}

impl ProximateShellError {
    fn start(self) -> ShellError {
        ShellError {
            cause: None,
            error: self,
        }
    }
}

impl ToDebug for ProximateShellError {
    fn fmt_debug(&self, f: &mut fmt::Formatter, _source: &str) -> fmt::Result {
        // TODO: Custom debug for inner spans
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellDiagnostic {
    pub(crate) diagnostic: Diagnostic<Span>,
}

impl ShellDiagnostic {
    #[allow(unused)]
    pub(crate) fn simple_diagnostic(
        span: impl Into<Span>,
        source: impl Into<String>,
    ) -> ShellDiagnostic {
        use language_reporting::*;

        let span = span.into();
        let source = source.into();

        let diagnostic =
            Diagnostic::new(Severity::Error, "Parse error").with_label(Label::new_primary(span));

        ShellDiagnostic { diagnostic }
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
    error: Value,
}

impl std::fmt::Display for ShellError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self.error {
            ProximateShellError::String(s) => write!(f, "{}", &s.title),
            ProximateShellError::MissingValue { .. } => write!(f, "MissingValue"),
            ProximateShellError::InvalidCommand { .. } => write!(f, "InvalidCommand"),
            ProximateShellError::TypeError { .. } => write!(f, "TypeError"),
            ProximateShellError::RangeError { .. } => write!(f, "RangeError"),
            ProximateShellError::SyntaxError { .. } => write!(f, "SyntaxError"),
            ProximateShellError::MissingProperty { .. } => write!(f, "MissingProperty"),
            ProximateShellError::ArgumentError { .. } => write!(f, "ArgumentError"),
            ProximateShellError::Diagnostic(_) => write!(f, "<diagnostic>"),
            ProximateShellError::CoerceError { .. } => write!(f, "CoerceError"),
        }
    }
}

impl std::error::Error for ShellError {}

impl std::convert::From<Box<dyn std::error::Error>> for ShellError {
    fn from(input: Box<dyn std::error::Error>) -> ShellError {
        ProximateShellError::String(StringError {
            title: format!("{}", input),
            error: Value::nothing(),
        })
        .start()
    }
}

impl std::convert::From<std::io::Error> for ShellError {
    fn from(input: std::io::Error) -> ShellError {
        ProximateShellError::String(StringError {
            title: format!("{}", input),
            error: Value::nothing(),
        })
        .start()
    }
}

impl std::convert::From<subprocess::PopenError> for ShellError {
    fn from(input: subprocess::PopenError) -> ShellError {
        ProximateShellError::String(StringError {
            title: format!("{}", input),
            error: Value::nothing(),
        })
        .start()
    }
}

impl std::convert::From<serde_yaml::Error> for ShellError {
    fn from(input: serde_yaml::Error) -> ShellError {
        ProximateShellError::String(StringError {
            title: format!("{:?}", input),
            error: Value::nothing(),
        })
        .start()
    }
}

impl std::convert::From<toml::ser::Error> for ShellError {
    fn from(input: toml::ser::Error) -> ShellError {
        ProximateShellError::String(StringError {
            title: format!("{:?}", input),
            error: Value::nothing(),
        })
        .start()
    }
}

impl std::convert::From<serde_json::Error> for ShellError {
    fn from(input: serde_json::Error) -> ShellError {
        ProximateShellError::String(StringError {
            title: format!("{:?}", input),
            error: Value::nothing(),
        })
        .start()
    }
}

impl std::convert::From<regex::Error> for ShellError {
    fn from(input: regex::Error) -> ShellError {
        ProximateShellError::String(StringError {
            title: format!("{:?}", input),
            error: Value::nothing(),
        })
        .start()
    }
}

impl std::convert::From<Box<dyn std::error::Error + Send + Sync>> for ShellError {
    fn from(input: Box<dyn std::error::Error + Send + Sync>) -> ShellError {
        ProximateShellError::String(StringError {
            title: format!("{:?}", input),
            error: Value::nothing(),
        })
        .start()
    }
}

pub trait ShellErrorUtils<T> {
    fn unwrap_error(self, desc: impl Into<String>) -> Result<T, ShellError>;
}

impl<T> ShellErrorUtils<Tagged<T>> for Option<Tagged<T>> {
    fn unwrap_error(self, desc: impl Into<String>) -> Result<Tagged<T>, ShellError> {
        match self {
            Some(value) => Ok(value),
            None => Err(ShellError::missing_value(None, desc.into())),
        }
    }
}

pub fn ranged<T>(
    input: Option<T>,
    expected: impl Into<String>,
    actual: Tagged<impl fmt::Debug>,
) -> Result<T, ShellError> {
    match input {
        Some(v) => Ok(v),
        None => Err(ShellError::range_error(expected, actual)),
    }
}
