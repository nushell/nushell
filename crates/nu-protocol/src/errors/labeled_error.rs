use super::{ShellError, shell_error::io::IoError};
use crate::{FromValue, IntoValue, Span, Type, Value, record};
use miette::{Diagnostic, LabeledSpan, NamedSource, SourceSpan};
use serde::{Deserialize, Serialize};
use std::{fmt, fs};

// # use nu_protocol::{FromValue, Value, ShellError, record, Span};

/// A very generic type of error used for interfacing with external code, such as scripts and
/// plugins.
///
/// This generally covers most of the interface of [`miette::Diagnostic`], but with types that are
/// well-defined for our protocol.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct LabeledError {
    /// The main message for the error.
    pub msg: String,
    /// Labeled spans attached to the error, demonstrating to the user where the problem is.
    #[serde(default)]
    pub labels: Box<Vec<ErrorLabel>>,
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
                .map(|label| ErrorLabel {
                    text: label.label().unwrap_or("").into(),
                    span: Span::new(label.offset(), label.offset() + label.len()),
                })
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

impl From<ErrorLabel> for LabeledSpan {
    fn from(val: ErrorLabel) -> Self {
        LabeledSpan::new(
            (!val.text.is_empty()).then_some(val.text),
            val.span.start,
            val.span.end - val.span.start,
        )
    }
}

impl From<ErrorLabel> for SourceSpan {
    fn from(val: ErrorLabel) -> Self {
        SourceSpan::new(val.span.start.into(), val.span.end - val.span.start)
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

/// Optionally named error source
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorSource {
    name: Option<String>,
    text: Option<String>,
    path: Option<String>,
}

impl ErrorSource {
    pub fn new(name: Option<String>, text: String) -> Self {
        Self {
            name,
            text: Some(text),
            path: None,
        }
    }
}

impl From<ErrorSource> for NamedSource<String> {
    fn from(value: ErrorSource) -> Self {
        let name = value.name.unwrap_or_default();
        match value {
            ErrorSource {
                text: Some(text),
                path: None,
                ..
            } => NamedSource::new(name, text),
            ErrorSource {
                text: None,
                path: Some(path),
                ..
            } => {
                let text = fs::read_to_string(&path).unwrap_or_default();
                NamedSource::new(path, text)
            }
            _ => NamedSource::new(name, "".into()),
        }
    }
}

impl FromValue for ErrorSource {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        let record = v.clone().into_record()?;
        let name = record
            .get("name")
            .and_then(|s| String::from_value(s.clone()).ok());
        // let name = String::from_value(record.get("name").unwrap().clone()).ok();

        let text = if let Some(text) = record.get("text") {
            String::from_value(text.clone()).ok()
        } else {
            None
        };
        let path = if let Some(path) = record.get("path") {
            String::from_value(path.clone()).ok()
        } else {
            None
        };

        match (text, path) {
            // Prioritize not reading from a file and using the text raw
            (text @ Some(_), _) => Ok(ErrorSource {
                name,
                text,
                path: None,
            }),
            (_, path @ Some(_)) => Ok(ErrorSource {
                name: path.clone(),
                text: None,
                path,
            }),
            _ => Err(ShellError::CantConvert {
                to_type: Self::expected_type().to_string(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
    fn expected_type() -> crate::Type {
        Type::Record(
            vec![
                ("name".into(), Type::String),
                ("text".into(), Type::String),
                ("path".into(), Type::String),
            ]
            .into(),
        )
    }
}

impl IntoValue for ErrorSource {
    fn into_value(self, span: Span) -> Value {
        match self {
            Self {
                name: Some(name),
                text: Some(text),
                ..
            } => record! {
                "name" => Value::string(name, span),
                "text" => Value::string(text, span),
            },
            Self {
                text: Some(text), ..
            } => record! {
                "text" => Value::string(text, span)
            },
            Self {
                name: Some(name),
                path: Some(path),
                ..
            } => record! {
                "name" => Value::string(name, span),
                "path" => Value::string(path, span),
            },
            Self {
                path: Some(path), ..
            } => record! {
                "path" => Value::string(path, span),
            },
            _ => record! {},
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

impl Diagnostic for LabeledError {
    fn code<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        self.code.as_ref().map(Box::new).map(|b| b as _)
    }

    fn help<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        self.help.as_ref().map(Box::new).map(|b| b as _)
    }

    fn url<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        self.url.as_ref().map(Box::new).map(|b| b as _)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        Some(Box::new(
            self.labels.iter().map(|label| label.clone().into()),
        ))
    }

    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn Diagnostic> + 'a>> {
        Some(Box::new(self.inner.iter().map(|r| r as _)))
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

/// Default number of context bytes on each side of an error span when truncating source
/// for diagnostics.
pub const DEFAULT_ERROR_CONTEXT: usize = 4096;

/// Truncates a source string to a bounded window around an error span.
///
/// Takes `context` bytes on each side of the error location. Returns the
/// truncated source and an adjusted span that is relative to the truncated window.
/// This prevents unbounded memory usage when error diagnostics embed large source files.
///
/// Slicing is safe on multi-byte UTF-8: window boundaries are adjusted to char boundaries.
///
/// For multi-line inputs the window expands to the nearest line boundaries so that line
/// numbers in the diagnostic output are consistent. For single-line (minified) inputs
/// with a large requested context, the window is clamped to a smaller focused snippet
/// since a wall of unbroken text is not helpful.
///
/// Use with `Span::try_from_row_column` when you only have (row, col) from the parser,
/// or directly when you already have the byte offset from the parser.
pub fn truncated_source_window(input: &str, byte_span: Span, context: usize) -> (String, Span) {
    let mid = (byte_span.start + byte_span.end) / 2;

    // Detect single-line (minified) input.  When the caller asks for a large context
    // but the input has no newlines nearby, a wall of unbroken text is useless — clamp
    // to a tight window focused on the error location.
    const TIGHT_CONTEXT: usize = 128;
    let is_single_line = if context > TIGHT_CONTEXT {
        let probe_start = input.floor_char_boundary(mid.saturating_sub(TIGHT_CONTEXT));
        let probe_end = input.ceil_char_boundary(input.len().min(mid + TIGHT_CONTEXT));
        !input[probe_start..probe_end].contains('\n')
    } else {
        false
    };
    let effective = if is_single_line {
        TIGHT_CONTEXT
    } else {
        context
    };

    let mut window_start = mid.saturating_sub(effective);
    let mut window_end = input.len().min(mid + effective);

    // Adjust to char boundaries to avoid panicking on multi-byte UTF-8
    window_start = input.floor_char_boundary(window_start);
    window_end = input.ceil_char_boundary(window_end);

    if !is_single_line && context > TIGHT_CONTEXT {
        // Multi-line with large context: round to nearest line boundaries for
        // proper line-number display.  Guard against blowing up by 2x context.
        window_start = if let Some(pos) = input[..window_start].rfind('\n') {
            let line_start = pos + 1;
            if window_start - line_start <= context * 2 {
                line_start
            } else {
                window_start
            }
        } else {
            window_start
        };
        window_end = if let Some(pos) = input[window_end..].find('\n') {
            let line_end = window_end + pos + 1;
            if line_end - window_end <= context * 2 {
                line_end
            } else {
                window_end
            }
        } else {
            window_end
        };
    }

    let truncated = input[window_start..window_end].to_string();
    let adjusted_span = Span::new(
        byte_span.start.saturating_sub(window_start),
        byte_span.end.saturating_sub(window_start),
    );
    (truncated, adjusted_span)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncated_source_window_middle() {
        // 40 bytes of padding on each side, "ERROR" spanning bytes 40..45
        // mid = (40+45)/2 = 42
        // window_start = 42-8 = 34, window_end = min(85,42+8) = 50
        let input = format!("{:a<40}ERROR{:b<40}", "", "");
        assert_eq!(input.len(), 85);
        let byte_span = Span::new(40, 45);
        let (src, span) = truncated_source_window(&input, byte_span, 8);
        assert!(
            src.contains("ERROR"),
            "truncated source should contain the error"
        );
        assert_eq!(span.start, 6, "40 - 34 = 6");
        assert_eq!(span.end, 11, "45 - 34 = 11");
    }

    #[test]
    fn truncated_source_window_near_start() {
        // mid = (0+4)/2 = 2
        // window_start = 2-8 = 0 (saturated), window_end = min(80, 2+8) = 10
        let input = format!("{:x<80}", "");
        let byte_span = Span::new(0, 4);
        let (src, span) = truncated_source_window(&input, byte_span, 8);
        assert_eq!(span.start, 0, "0 - 0 = 0");
        assert_eq!(span.end, 4, "4 - 0 = 4");
        assert_eq!(src.len(), 10, "window [0, 10) is 10 bytes");
    }

    #[test]
    fn truncated_source_window_near_end() {
        // mid = (76+80)/2 = 78
        // window_start = 78-8 = 70, window_end = min(80, 78+8) = 80
        let input = format!("{:x<80}", "");
        let byte_span = Span::new(76, 80);
        let (src, span) = truncated_source_window(&input, byte_span, 8);
        assert_eq!(span.start, 6, "76 - 70 = 6");
        assert_eq!(span.end, 10, "80 - 70 = 10");
        assert_eq!(src.len(), 10, "window [70, 80) is 10 bytes");
    }

    #[test]
    fn truncated_source_window_small_input() {
        let input = "small";
        let byte_span = Span::new(2, 4);
        let (src, span) = truncated_source_window(input, byte_span, 100);
        // Input is smaller than context, so window should be the entire input
        assert_eq!(
            src, "small",
            "should be the full input when context > input.len()"
        );
        assert_eq!(span.start, 2, "adjusted span start should match original");
        assert_eq!(span.end, 4, "adjusted span end should match original");
    }

    #[test]
    fn truncated_source_window_span_adjustment() {
        // Use XXXXX as the error marker to avoid typos-tool false
        // positives on an error-word adjacent to other characters.
        let input = "aaaaaaaaaaXXXXXbbbbbbbbbb"; // 10 a's, 5 X's, 10 b's = 25 bytes
        // Error from byte 10 to byte 15 -> "XXXXX"
        let byte_span = Span::new(10, 15);
        let (src, span) = truncated_source_window(input, byte_span, 5);
        // Window: mid=12, window_start = 12-5 = 7, window_end = 12+5 = 17
        // src = input[7..17] = "aaaXXXXXbb" (3 a's + 5 X's + 2 b's = 10 bytes)
        assert_eq!(src.len(), 10, "window should be 10 bytes");
        assert!(src.starts_with("aaa"), "window should start with aaa");
        assert!(src.ends_with("bb"), "window should end with bb");
        assert!(
            src.contains("XXXXX"),
            "window should contain the error marker"
        );
        // Adjusted: byte_span.start - window_start = 10-7 = 3
        assert_eq!(
            span.start, 3,
            "adjusted start should be original - window_start"
        );
        assert_eq!(
            span.end, 8,
            "adjusted end should be original - window_start"
        );
        assert_eq!(
            &src[3..8],
            "XXXXX",
            "error marker should be at the right adjusted position"
        );
    }

    #[test]
    fn truncated_source_window_zero_width_span() {
        let input = "abcdefghijklmnopqrstuvwxyz";
        let byte_span = Span::new(13, 13); // middle of alphabet
        let (src, span) = truncated_source_window(input, byte_span, 5);
        assert_eq!(
            span.start, span.end,
            "zero-width span should stay zero-width"
        );
        assert!(src.len() <= 11, "window should be bounded");
    }

    #[test]
    fn truncated_source_window_multibyte_utf8() {
        // Chinese chars are 3 bytes each; slicing at arbitrary byte offsets must not panic
        let input = "你好世界ERROR世界";
        // "ERROR" starts at byte 12 (4 chars × 3 bytes)
        let byte_span = Span::new(12, 17);
        let (src, span) = truncated_source_window(input, byte_span, 3);
        assert!(
            src.contains("ERROR"),
            "window must contain the error region"
        );
        assert_eq!(
            &src[span.start..span.end],
            "ERROR",
            "adjusted span must slice correctly"
        );
    }

    #[test]
    fn truncated_source_window_multibyte_utf8_boundary_crossing() {
        // Force window bounds into the middle of multi-byte chars
        // "aaaaa" (5 bytes) + "你好世界" (12 bytes) + "ERROR" (5 bytes) + "世界你好" (12 bytes)
        let input = "aaaaa你好世界ERROR世界你好";
        // "ERROR" at bytes 17..22
        let byte_span = Span::new(17, 22);
        // context=8 should give enough room while crossing multi-byte boundaries
        let (src, span) = truncated_source_window(input, byte_span, 8);
        assert!(
            src.contains("ERROR"),
            "window must contain the error region"
        );
        assert_eq!(&src[span.start..span.end], "ERROR");
    }

    #[test]
    fn truncated_source_window_single_line_minified() {
        // Simulate a minified JSON file: one giant line, error near the end.
        let mut input = String::new();
        input.push_str(&"\"key\":\"value\",".repeat(500)); // 14 bytes each
        let err_byte = input.len(); // byte right after the valid part
        input.push_str("\"broken"); // syntax error starts here
        let byte_span = Span::new(err_byte, err_byte + 1); // the opening quote of "broken
        let (src, span) = truncated_source_window(&input, byte_span, DEFAULT_ERROR_CONTEXT);
        // The window should be tight (no wall of text) for single-line input.
        assert!(
            src.len() < 1000,
            "single-line window should be tight, got {} bytes",
            src.len()
        );
        assert_eq!(
            &src[span.start..span.end],
            "\"",
            "should point at the opening quote"
        );
    }

    #[test]
    fn truncated_source_window_multiline_uses_full_context() {
        // Multi-line input should get the full context window with line boundaries.
        let mut input = String::new();
        for i in 0..200 {
            use std::fmt::Write;
            writeln!(&mut input, "line {i}").unwrap();
        }
        input.push_str("ERROR here\nlast line");
        // "ERROR here" starts at some point in the file
        let err_offset = input.find("ERROR").expect("ERROR should be in input");
        let byte_span = Span::new(err_offset, err_offset + 5);
        // Use a generous context to show it's not truncated to tight window
        let (src, span) = truncated_source_window(&input, byte_span, DEFAULT_ERROR_CONTEXT);
        // Multi-line: should contain the whole lines around the error, not tight
        assert!(
            src.len() > 1000,
            "multi-line window should be large, got {} bytes",
            src.len()
        );
        assert!(src.contains("ERROR"), "should contain the error region");
        assert_eq!(&src[span.start..span.end], "ERROR");
    }
}
