//! Parse diagnostics.
//!
//! Every problem the parser reports is a [`Diagnostic`]: a kind, a span, the
//! stack of grammar contexts that were active, and optional help. [`ParseError`]
//! is the error type returned by [`crate::parse`]; it holds one or more
//! diagnostics (the parser recovers at statement boundaries and keeps going).

use std::fmt;

use crate::span::{LineIndex, Span};

/// What went wrong.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub enum ErrorKind {
    /// Something specific was expected at this position.
    Expected(&'static str),
    /// A keyword was expected at this position.
    ExpectedKeyword(&'static str),
    /// A delimiter was opened but never closed.
    Unclosed {
        /// The closing delimiter that is missing, e.g. `)`.
        delimiter: &'static str,
        /// Where the matching opening delimiter is.
        open: Span,
    },
    /// A closing delimiter with no matching opener.
    Unbalanced {
        /// The closing delimiter that was found.
        found: &'static str,
        /// The opener it would need.
        expected: &'static str,
    },
    /// Input ended while a construct was still incomplete.
    UnexpectedEof(&'static str),
    /// Tokens were found where the construct should have ended.
    ExtraTokens,
    /// A literal was recognised but malformed.
    InvalidLiteral {
        /// The literal kind, e.g. `"int"` or `"string"`.
        kind: &'static str,
        /// Details.
        message: String,
    },
    /// A type name that Nushell does not know.
    UnknownType(String),
    /// An operator that Nushell does not have.
    UnknownOperator(String),
    /// A bash-ism that Nushell rejects, with the Nushell spelling.
    ShellSyntax {
        /// What was written.
        found: &'static str,
        /// What Nushell wants instead.
        use_instead: &'static str,
    },
    /// A statement keyword was used in a position where it is not allowed.
    KeywordInPipeline(String),
    /// Something else, described in prose.
    Message(String),
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::Expected(what) => write!(f, "expected {what}"),
            ErrorKind::ExpectedKeyword(kw) => write!(f, "expected keyword `{kw}`"),
            ErrorKind::Unclosed { delimiter, .. } => write!(f, "unclosed delimiter: expected `{delimiter}`"),
            ErrorKind::Unbalanced { found, expected } => {
                write!(f, "unbalanced delimiter: found `{found}` without a matching `{expected}`")
            }
            ErrorKind::UnexpectedEof(what) => write!(f, "unexpected end of input, expected {what}"),
            ErrorKind::ExtraTokens => write!(f, "extra tokens after the end of the expression"),
            ErrorKind::InvalidLiteral { kind, message } => write!(f, "invalid {kind} literal: {message}"),
            ErrorKind::UnknownType(name) => write!(f, "unknown type `{name}`"),
            ErrorKind::UnknownOperator(op) => write!(f, "unknown operator `{op}`"),
            ErrorKind::ShellSyntax { found, use_instead } => {
                write!(f, "`{found}` is not Nushell syntax; use `{use_instead}` instead")
            }
            ErrorKind::KeywordInPipeline(kw) => {
                write!(f, "`{kw}` is a statement and cannot be used inside a pipeline")
            }
            ErrorKind::Message(msg) => f.write_str(msg),
        }
    }
}

/// One parse problem with its location.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Diagnostic {
    /// What went wrong.
    pub kind: ErrorKind,
    /// Where. May be empty (a point) when the problem is a missing token.
    pub span: Span,
    /// Grammar constructs that were being parsed, innermost first
    /// (e.g. `["closure parameters", "closure", "pipeline"]`).
    pub context: Vec<&'static str>,
    /// Optional advice.
    pub help: Option<String>,
}

impl Diagnostic {
    /// Create a diagnostic.
    pub fn new(kind: ErrorKind, span: Span) -> Self {
        Self { kind, span, context: Vec::new(), help: None }
    }

    /// `expected <what>` at `span`.
    pub fn expected(what: &'static str, span: Span) -> Self {
        Self::new(ErrorKind::Expected(what), span)
    }

    /// A free-form message at `span`.
    pub fn message(msg: impl Into<String>, span: Span) -> Self {
        Self::new(ErrorKind::Message(msg.into()), span)
    }

    /// Attach help text.
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Push a context label (innermost first).
    pub fn with_context(mut self, ctx: &'static str) -> Self {
        self.context.push(ctx);
        self
    }

    /// The outermost-but-one context, useful for messages like "in closure".
    pub fn innermost_context(&self) -> Option<&'static str> {
        self.context.first().copied()
    }

    /// Render with source context: `error: … --> name:line:col` followed by the
    /// offending line and a caret marker.
    pub fn render(&self, source: &str, name: Option<&str>) -> String {
        let index = LineIndex::new(source);
        let mut out = String::new();
        render_into(&mut out, self, source, name, &index);
        out
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {}", self.kind, self.span)?;
        if let Some(ctx) = self.context.first() {
            write!(f, " (while parsing {ctx})")?;
        }
        Ok(())
    }
}

impl std::error::Error for Diagnostic {}

/// The error returned by [`crate::parse`]: one or more diagnostics, in source order.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ParseError {
    /// All problems found. Never empty.
    pub diagnostics: Vec<Diagnostic>,
}

impl ParseError {
    /// Wrap a non-empty list of diagnostics.
    ///
    /// # Panics
    /// Panics if `diagnostics` is empty.
    pub fn new(diagnostics: Vec<Diagnostic>) -> Self {
        assert!(!diagnostics.is_empty(), "ParseError needs at least one diagnostic");
        Self { diagnostics }
    }

    /// The first (earliest) diagnostic.
    pub fn primary(&self) -> &Diagnostic {
        &self.diagnostics[0]
    }

    /// Render every diagnostic with source context, separated by blank lines.
    pub fn render(&self, source: &str, name: Option<&str>) -> String {
        let index = LineIndex::new(source);
        let mut out = String::new();
        for (i, d) in self.diagnostics.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            render_into(&mut out, d, source, name, &index);
        }
        out
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.diagnostics.len() {
            1 => write!(f, "{}", self.diagnostics[0]),
            n => write!(f, "{} ({} more errors)", self.diagnostics[0], n - 1),
        }
    }
}

impl std::error::Error for ParseError {}

impl From<Diagnostic> for ParseError {
    fn from(d: Diagnostic) -> Self {
        ParseError { diagnostics: vec![d] }
    }
}

fn render_into(out: &mut String, d: &Diagnostic, source: &str, name: Option<&str>, index: &LineIndex) {
    use std::fmt::Write as _;
    let start = index.line_col(d.span.start, source);
    let name = name.unwrap_or("<input>");
    let _ = writeln!(out, "error: {}", d.kind);
    let _ = writeln!(out, "  --> {name}:{start}");
    let line_idx = start.line - 1;
    let range = index.line_range(line_idx, source);
    let line_text = &source[range.clone()];
    let gutter = format!("{}", start.line);
    let pad = " ".repeat(gutter.len());
    let _ = writeln!(out, "{pad} |");
    let _ = writeln!(out, "{gutter} | {line_text}");
    // Caret line: count characters up to the span start on this line.
    let col0 = source[range.start..d.span.start.min(range.end).max(range.start)].chars().count();
    let end_on_line = d.span.end.min(range.end).max(d.span.start.min(range.end));
    let width = source[d.span.start.min(range.end)..end_on_line].chars().count().max(1);
    let _ = writeln!(out, "{pad} | {}{}", " ".repeat(col0), "^".repeat(width));
    if let ErrorKind::Unclosed { open, .. } = &d.kind {
        let open_pos = index.line_col(open.start, source);
        let _ = writeln!(out, "{pad} = note: opened at {open_pos}");
    }
    if let Some(ctx) = d.context.first() {
        let _ = writeln!(out, "{pad} = while parsing {ctx}");
    }
    if let Some(help) = &d.help {
        let _ = writeln!(out, "{pad} = help: {help}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_with_caret() {
        let src = "let x = [1 2\nlet y = 2";
        let d = Diagnostic::new(ErrorKind::Unclosed { delimiter: "]", open: Span::new(8, 9) }, Span::new(12, 12))
            .with_context("list");
        let text = d.render(src, Some("t.nu"));
        assert!(text.contains("error: unclosed delimiter: expected `]`"));
        assert!(text.contains("--> t.nu:1:13"));
        assert!(text.contains("1 | let x = [1 2"));
        assert!(text.contains("opened at 1:9"));
        assert!(text.contains("while parsing list"));
    }
}
