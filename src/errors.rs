#[allow(unused)]
use crate::prelude::*;

use crate::parser::{Span, Spanned};
use ansi_term::Color;
use derive_new::new;
use language_reporting::{Diagnostic, Label, Severity};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Eq, PartialEq, Clone, Ord, PartialOrd, Serialize, Deserialize)]
pub enum Description {
    Source(Spanned<String>),
    Synthetic(String),
}

impl Description {
    pub fn from(item: Spanned<impl Into<String>>) -> Description {
        match item {
            Spanned {
                span: Span { start: 0, end: 0 },
                item,
            } => Description::Synthetic(item.into()),
            Spanned { span, item } => Description::Source(Spanned::from_item(item.into(), span)),
        }
    }
}

impl Description {
    fn into_label(self) -> Result<Label<Span>, String> {
        match self {
            Description::Source(s) => Ok(Label::new_primary(s.span).with_message(s.item)),
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

pub fn labelled(
    span: impl Into<Option<Span>>,
    heading: &'a str,
    span_message: &'a str,
) -> impl FnOnce(ShellError) -> ShellError + 'a {
    let span = span.into();

    move |_| ShellError::maybe_labeled_error(heading, span_message, span)
}

#[derive(Debug, Eq, PartialEq, Clone, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ShellError {
    error: ProximateShellError,
    cause: Option<Box<ProximateShellError>>,
}

impl ShellError {
    crate fn type_error(
        expected: impl Into<String>,
        actual: Spanned<impl Into<String>>,
    ) -> ShellError {
        ProximateShellError::TypeError {
            expected: expected.into(),
            actual: actual.map(|i| Some(i.into())),
        }
        .start()
    }

    crate fn coerce_error(
        left: Spanned<impl Into<String>>,
        right: Spanned<impl Into<String>>,
    ) -> ShellError {
        ProximateShellError::CoerceError {
            left: left.map(|l| l.into()),
            right: right.map(|r| r.into()),
        }
        .start()
    }

    crate fn missing_property(subpath: Description, expr: Description) -> ShellError {
        ProximateShellError::MissingProperty { subpath, expr }.start()
    }

    crate fn argument_error(
        command: impl Into<String>,
        kind: ArgumentError,
        span: Span,
    ) -> ShellError {
        ProximateShellError::ArgumentError {
            command: command.into(),
            error: kind,
            span: span,
        }
        .start()
    }

    crate fn parse_error(
        error: nom::Err<(nom_locate::LocatedSpan<&str>, nom::error::ErrorKind)>,
    ) -> ShellError {
        use language_reporting::*;

        match error {
            nom::Err::Incomplete(_) => unreachable!(),
            nom::Err::Failure(span) | nom::Err::Error(span) => {
                let diagnostic = Diagnostic::new(Severity::Error, format!("Parse Error"))
                    .with_label(Label::new_primary(Span::from(span.0)));

                ShellError::diagnostic(diagnostic)
            }
        }
    }

    crate fn diagnostic(diagnostic: Diagnostic<Span>) -> ShellError {
        ProximateShellError::Diagnostic(ShellDiagnostic { diagnostic }).start()
    }

    crate fn to_diagnostic(self) -> Diagnostic<Span> {
        match self.error {
            ProximateShellError::String(StringError { title, .. }) => {
                Diagnostic::new(Severity::Error, title)
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
                        "{} requires {}",
                        Color::Cyan.paint(command),
                        Color::Green.bold().paint(name)
                    ),
                )
                .with_label(Label::new_primary(span)),

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
                    Spanned {
                        item: Some(actual),
                        span,
                    },
            } => Diagnostic::new(Severity::Error, "Type Error").with_label(
                Label::new_primary(span)
                    .with_message(format!("Expected {}, found {}", expected, actual)),
            ),

            ProximateShellError::TypeError {
                expected,
                actual: Spanned { item: None, span },
            } => Diagnostic::new(Severity::Error, "Type Error")
                .with_label(Label::new_primary(span).with_message(expected)),

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
                    .with_label(Label::new_primary(left.span).with_message(left.item))
                    .with_label(Label::new_secondary(right.span).with_message(right.item))
            }
        }
    }

    crate fn labeled_error(
        msg: impl Into<String>,
        label: impl Into<String>,
        span: Span,
    ) -> ShellError {
        ShellError::diagnostic(
            Diagnostic::new(Severity::Error, msg.into())
                .with_label(Label::new_primary(span).with_message(label.into())),
        )
    }

    crate fn maybe_labeled_error(
        msg: impl Into<String>,
        label: impl Into<String>,
        span: Option<Span>,
    ) -> ShellError {
        match span {
            Some(span) => ShellError::diagnostic(
                Diagnostic::new(Severity::Error, msg.into())
                    .with_label(Label::new_primary(span).with_message(label.into())),
            ),
            None => ShellError::string(msg),
        }
    }

    pub fn string(title: impl Into<String>) -> ShellError {
        ProximateShellError::String(StringError::new(title.into(), Value::nothing())).start()
    }

    crate fn unimplemented(title: impl Into<String>) -> ShellError {
        ShellError::string(&format!("Unimplemented: {}", title.into()))
    }

    crate fn unexpected(title: impl Into<String>) -> ShellError {
        ShellError::string(&format!("Unexpected: {}", title.into()))
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Ord, PartialOrd, Serialize, Deserialize)]
pub enum ProximateShellError {
    String(StringError),
    TypeError {
        expected: String,
        actual: Spanned<Option<String>>,
    },
    MissingProperty {
        subpath: Description,
        expr: Description,
    },
    ArgumentError {
        command: String,
        error: ArgumentError,
        span: Span,
    },
    Diagnostic(ShellDiagnostic),
    CoerceError {
        left: Spanned<String>,
        right: Spanned<String>,
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

#[derive(Debug, Clone)]
pub struct ShellDiagnostic {
    crate diagnostic: Diagnostic<Span>,
}

impl ShellDiagnostic {
    #[allow(unused)]
    crate fn simple_diagnostic(
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

impl Serialize for ShellDiagnostic {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        "<diagnostic>".serialize(serializer)
    }
}

impl Deserialize<'de> for ShellDiagnostic {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(ShellDiagnostic {
            diagnostic: Diagnostic::new(
                language_reporting::Severity::Error,
                "deserialize not implemented for ShellDiagnostic",
            ),
        })
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
            ProximateShellError::TypeError { .. } => write!(f, "TypeError"),
            ProximateShellError::MissingProperty { .. } => write!(f, "MissingProperty"),
            ProximateShellError::ArgumentError { .. } => write!(f, "ArgumentError"),
            ProximateShellError::Diagnostic(_) => write!(f, "<diagnostic>"),
            ProximateShellError::CoerceError { .. } => write!(f, "CoerceError"),
        }
    }
}

impl std::error::Error for ShellError {}

impl std::convert::From<std::io::Error> for ShellError {
    fn from(input: std::io::Error) -> ShellError {
        ProximateShellError::String(StringError {
            title: format!("{}", input),
            error: Value::nothing(),
        })
        .start()
    }
}

impl std::convert::From<futures_sink::VecSinkError> for ShellError {
    fn from(_input: futures_sink::VecSinkError) -> ShellError {
        ProximateShellError::String(StringError {
            title: format!("Unexpected Vec Sink Error"),
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

impl std::convert::From<toml::ser::Error> for ShellError {
    fn from(input: toml::ser::Error) -> ShellError {
        ProximateShellError::String(StringError {
            title: format!("{:?}", input),
            error: Value::nothing(),
        })
        .start()
    }
}
