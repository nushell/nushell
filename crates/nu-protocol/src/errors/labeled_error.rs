use std::fmt;

use miette::Diagnostic;
use serde::{Deserialize, Serialize};

use crate::Span;

use super::ShellError;

/// A very generic type of error used for interfacing with external code, such as scripts and
/// plugins.
///
/// This generally covers most of the interface of [`miette::Diagnostic`], but with types that are
/// well-defined for our protocol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LabeledError {
    /// The main message for the error.
    pub msg: String,
    /// Labeled spans attached to the error, demonstrating to the user where the problem is.
    #[serde(default)]
    pub labels: Vec<ErrorLabel>,
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
    pub inner: Vec<LabeledError>,
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
    pub fn new(msg: impl Into<String>) -> LabeledError {
        LabeledError {
            msg: msg.into(),
            labels: vec![],
            code: None,
            url: None,
            help: None,
            inner: vec![],
        }
    }

    /// Add a labeled span to the error to demonstrate to the user where the problem is.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use nu_protocol::{LabeledError, Span};
    /// # let span = Span::test_data();
    /// let error = LabeledError::new("An error")
    ///     .with_label("happened here", span);
    /// assert_eq!("happened here", &error.labels[0].text);
    /// assert_eq!(span, error.labels[0].span);
    /// ```
    pub fn with_label(mut self, text: impl Into<String>, span: Span) -> Self {
        self.labels.push(ErrorLabel {
            text: text.into(),
            span,
        });
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
    /// # use nu_protocol::LabeledError;
    /// let error = LabeledError::new("An error")
    ///     .with_inner(LabeledError::new("out of coolant"));
    /// assert_eq!(LabeledError::new("out of coolant"), error.inner[0]);
    /// ```
    pub fn with_inner(mut self, inner: impl Into<LabeledError>) -> Self {
        self.inner.push(inner.into());
        self
    }

    /// Create a [`LabeledError`] from a type that implements [`miette::Diagnostic`].
    ///
    /// # Example
    ///
    /// [`ShellError`] implements `miette::Diagnostic`:
    ///
    /// ```rust
    /// # use nu_protocol::{ShellError, LabeledError};
    /// let error = LabeledError::from_diagnostic(&ShellError::IOError { msg: "error".into() });
    /// assert!(error.to_string().contains("I/O error"));
    /// ```
    pub fn from_diagnostic(diag: &(impl miette::Diagnostic + ?Sized)) -> LabeledError {
        LabeledError {
            msg: diag.to_string(),
            labels: diag
                .labels()
                .into_iter()
                .flatten()
                .map(|label| ErrorLabel {
                    text: label.label().unwrap_or("").into(),
                    span: Span::new(label.offset(), label.offset() + label.len()),
                })
                .collect(),
            code: diag.code().map(|s| s.to_string()),
            url: diag.url().map(|s| s.to_string()),
            help: diag.help().map(|s| s.to_string()),
            inner: diag
                .related()
                .into_iter()
                .flatten()
                .map(Self::from_diagnostic)
                .collect(),
        }
    }
}

/// A labeled span within a [`LabeledError`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorLabel {
    /// Text to show together with the span
    pub text: String,
    /// Span pointing at where the text references in the source
    pub span: Span,
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

impl Diagnostic for LabeledError {
    fn code<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        self.code.as_ref().map(Box::new).map(|b| b as _)
    }

    fn severity(&self) -> Option<miette::Severity> {
        None
    }

    fn help<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        self.help.as_ref().map(Box::new).map(|b| b as _)
    }

    fn url<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        self.url.as_ref().map(Box::new).map(|b| b as _)
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        None
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        Some(Box::new(self.labels.iter().map(|label| {
            miette::LabeledSpan::new_with_span(
                Some(label.text.clone()).filter(|s| !s.is_empty()),
                label.span,
            )
        })))
    }

    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn Diagnostic> + 'a>> {
        Some(Box::new(self.inner.iter().map(|r| r as _)))
    }

    fn diagnostic_source(&self) -> Option<&dyn Diagnostic> {
        None
    }
}

impl From<ShellError> for LabeledError {
    fn from(err: ShellError) -> Self {
        LabeledError::from_diagnostic(&err)
    }
}
