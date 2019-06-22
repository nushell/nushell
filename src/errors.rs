use crate::parser::lexer::{Span, SpannedToken};
#[allow(unused)]
use crate::prelude::*;
use derive_new::new;
use language_reporting::{Diagnostic, Label, Severity};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Eq, PartialEq, Clone, Ord, PartialOrd, Serialize, Deserialize)]
pub enum ShellError {
    String(StringError),
    TypeError(String),
    MissingProperty { subpath: String, expr: String },
    Diagnostic(ShellDiagnostic),
}

impl ShellError {
    crate fn parse_error(
        error: lalrpop_util::ParseError<usize, SpannedToken, ShellError>,
    ) -> ShellError {
        use lalrpop_util::ParseError;
        use language_reporting::*;

        match error {
            ParseError::UnrecognizedToken {
                token: (start, SpannedToken { token, .. }, end),
                expected,
            } => {
                let diagnostic = Diagnostic::new(
                    Severity::Error,
                    format!("Unexpected {:?}, expected {:?}", token, expected),
                )
                .with_label(Label::new_primary(Span::from((start, end))));

                ShellError::diagnostic(diagnostic)
            }
            ParseError::User { error } => error,
            other => ShellError::string(format!("{:?}", other)),
        }
    }

    crate fn diagnostic(diagnostic: Diagnostic<Span>) -> ShellError {
        ShellError::Diagnostic(ShellDiagnostic { diagnostic })
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

    crate fn string(title: impl Into<String>) -> ShellError {
        ShellError::String(StringError::new(title.into(), Value::nothing()))
    }

    crate fn unimplemented(title: impl Into<String>) -> ShellError {
        ShellError::string(&format!("Unimplemented: {}", title.into()))
    }

    crate fn copy_error(&self) -> ShellError {
        self.clone()
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
        match self {
            ShellError::String(s) => write!(f, "{}", &s.title),
            ShellError::TypeError { .. } => write!(f, "TypeError"),
            ShellError::MissingProperty { .. } => write!(f, "MissingProperty"),
            ShellError::Diagnostic(_) => write!(f, "<diagnostic>"),
        }
    }
}

impl std::error::Error for ShellError {}

impl std::convert::From<std::io::Error> for ShellError {
    fn from(input: std::io::Error) -> ShellError {
        ShellError::String(StringError {
            title: format!("{}", input),
            error: Value::nothing(),
        })
    }
}

impl std::convert::From<futures_sink::VecSinkError> for ShellError {
    fn from(_input: futures_sink::VecSinkError) -> ShellError {
        ShellError::String(StringError {
            title: format!("Unexpected Vec Sink Error"),
            error: Value::nothing(),
        })
    }
}

impl std::convert::From<subprocess::PopenError> for ShellError {
    fn from(input: subprocess::PopenError) -> ShellError {
        ShellError::String(StringError {
            title: format!("{}", input),
            error: Value::nothing(),
        })
    }
}

impl std::convert::From<nom::Err<(&str, nom::ErrorKind)>> for ShellError {
    fn from(input: nom::Err<(&str, nom::ErrorKind)>) -> ShellError {
        ShellError::String(StringError {
            title: format!("{:?}", input),
            error: Value::nothing(),
        })
    }
}

impl std::convert::From<toml::ser::Error> for ShellError {
    fn from(input: toml::ser::Error) -> ShellError {
        ShellError::String(StringError {
            title: format!("{:?}", input),
            error: Value::nothing(),
        })
    }
}
