use super::{ShellError, shell_error::io::IoError};
use crate::{FromValue, IntoValue, Record, Span, Type, Value, record};
use miette::{Diagnostic, LabeledSpan, SourceSpan};
use serde::{Deserialize, Serialize};
use std::fmt;

// # use nu_protocol::{FromValue, Value, ShellError, record, Span};

/// A very generic type of error used for interfacing with external code, such as scripts and
/// plugins.
///
/// This generally covers most of the interface of [`miette::Diagnostic`], but with types that are
/// well-defined for our protocol.
#[derive(Debug, Default, Diagnostic, Clone, PartialEq, Serialize, Deserialize)]
pub struct LabeledError {
    /// The main message for the error.
    pub msg: String,
    /// Labeled spans attached to the error, demonstrating to the user where the problem is.
    #[serde(default)]
    #[label(collection)]
    pub labels: Box<Vec<LabeledSpan>>,
    /// A unique machine- and search-friendly error code to associate to the error. (e.g.
    /// `nu::shell::missing_config_value`)
    #[serde(default)]
    pub code: Option<String>,
    /// A link to documentation about the error, used in conjunction with `code`
    #[serde(default)]
    pub url: Option<String>,
    /// Additional help for the error, usually a hint about what the user might try
    #[serde(default)]
    pub help: Option<String>,
    /// Errors that are related to or caused this error
    #[serde(default)]
    #[related]
    pub inner: Box<Vec<ShellError>>,
}

impl LabeledError {
    /// Create a new plain [`LabeledError`] with the given message.
    ///
    /// This is usually used builder-style with methods like [`.with_label()`](Self::with_label) to
    /// build an error.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use nu_protocol::LabeledError;
    /// let error = LabeledError::new("Something bad happened");
    /// assert_eq!("Something bad happened", error.to_string());
    /// ```
    pub fn new(msg: impl Into<String>) -> Self {
        Self {
            msg: msg.into(),
            ..Default::default()
        }
    }

    /// Add a labeled span to the error to demonstrate to the user where the problem is.
    pub fn with_label(mut self, text: impl Into<String>, span: Span) -> Self {
        self.labels.push(
            ErrorLabel {
                text: text.into(),
                span,
            }
            .into(),
        );
        self
    }

    /// Add a unique machine- and search-friendly error code to associate to the error. (e.g.
    /// `nu::shell::missing_config_value`)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use nu_protocol::LabeledError;
    /// let error = LabeledError::new("An error")
    ///     .with_code("my_product::error");
    /// assert_eq!(Some("my_product::error"), error.code.as_deref());
    /// ```
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Add a link to documentation about the error, used in conjunction with `code`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use nu_protocol::LabeledError;
    /// let error = LabeledError::new("An error")
    ///     .with_url("https://example.org/");
    /// assert_eq!(Some("https://example.org/"), error.url.as_deref());
    /// ```
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Add additional help for the error, usually a hint about what the user might try.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use nu_protocol::LabeledError;
    /// let error = LabeledError::new("An error")
    ///     .with_help("did you try turning it off and back on again?");
    /// assert_eq!(Some("did you try turning it off and back on again?"), error.help.as_deref());
    /// ```
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Add an error that is related to or caused this error.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use nu_protocol::{LabeledError, ShellError};
    /// let error = LabeledError::new("An error")
    ///     .with_inner(LabeledError::new("out of coolant"));
    /// let check: ShellError = LabeledError::new("out of coolant").into();
    /// assert_eq!(check, error.inner[0]);
    /// ```
    pub fn with_inner(mut self, inner: impl Into<ShellError>) -> Self {
        let inner_error: ShellError = inner.into();
        self.inner.push(inner_error);
        self
    }

    /// Create a [`LabeledError`] from a type that implements [`miette::Diagnostic`].
    ///
    /// # Example
    ///
    /// [`ShellError`] implements `miette::Diagnostic`:
    ///
    /// ```rust
    /// # use nu_protocol::{ShellError, LabeledError, shell_error::{self, io::IoError}, Span};
    /// #
    /// let error = LabeledError::from_diagnostic(
    ///     &ShellError::Io(IoError::new_with_additional_context(
    ///         shell_error::io::ErrorKind::from_std(std::io::ErrorKind::Other),
    ///         Span::test_data(),
    ///         None,
    ///         "some error"
    ///     ))
    /// );
    /// assert!(error.to_string().contains("I/O error"));
    /// ```
    pub fn from_diagnostic(diag: &(impl miette::Diagnostic + ?Sized)) -> Self {
        Self {
            msg: diag.to_string(),
            labels: diag
                .labels()
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .into(),
            code: diag.code().map(|s| s.to_string()),
            url: diag.url().map(|s| s.to_string()),
            help: diag.help().map(|s| s.to_string()),
            inner: diag
                .related()
                .into_iter()
                .flatten()
                .map(|i| Self::from_diagnostic(i).into())
                .collect::<Vec<_>>()
                .into(),
        }
    }
}

/// A labeled span within a [`LabeledError`].
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorLabel {
    /// Text to show together with the span
    pub text: String,
    /// Span pointing at where the text references in the source
    pub span: Span,
}

impl Into<LabeledSpan> for ErrorLabel {
    fn into(self) -> LabeledSpan {
        LabeledSpan::new(
            (!self.text.is_empty()).then_some(self.text),
            self.span.start.into(),
            self.span.end - self.span.start,
        )
    }
}

impl Into<SourceSpan> for ErrorLabel {
    fn into(self) -> SourceSpan {
        SourceSpan::new(self.span.start.into(), self.span.end - self.span.start)
    }
}

impl FromValue for ErrorLabel {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        let record = v.clone().into_record()?;
        let text = String::from_value(match record.get("text") {
            Some(val) => val.clone(),
            None => Value::string("", v.span()),
        })
        .unwrap_or("originates from here".into());
        let span = Span::from_value(match record.get("span") {
            Some(val) => val.clone(),
            // Maybe there's a better way...
            None => Value::record(
                record! {
                    "start" => Value::int(v.span().start as i64, v.span()),
                    "end" => Value::int(v.span().end as i64, v.span()),
                },
                v.span(),
            ),
        });

        match span {
            Ok(s) => Ok(Self { text, span: s }),
            Err(e) => Err(e),
        }
    }
    fn expected_type() -> crate::Type {
        Type::Record(
            vec![
                ("text".into(), Type::String),
                ("span".into(), Type::record()),
            ]
            .into(),
        )
    }
}

impl IntoValue for ErrorLabel {
    fn into_value(self, span: Span) -> Value {
        record! {
            "text" => Value::string(self.text, span),
            "span" => span.into_value(span),
        }
        .into_value(span)
    }
}

impl fmt::Display for LabeledError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.msg)
    }
}

impl std::error::Error for LabeledError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.inner.first().map(|r| r as _)
    }
}

impl From<ShellError> for LabeledError {
    fn from(err: ShellError) -> Self {
        Self::from_diagnostic(&err)
    }
}

impl From<IoError> for LabeledError {
    fn from(err: IoError) -> Self {
        Self::from_diagnostic(&err)
    }
}

impl From<Record> for LabeledError {
    fn from(_err: Record) -> Self {
        Self {
            msg: "foo".into(),
            ..Default::default()
        }
    }
}
