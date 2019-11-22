use crate::prelude::*;

use crate::parser::parse::parser::TracableContext;
use ansi_term::Color;
use derive_new::new;
use language_reporting::{Diagnostic, Label, Severity};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Range;

#[derive(Debug, Eq, PartialEq, Clone, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum Description {
    Source(Spanned<String>),
    Synthetic(String),
}

impl<T: Into<String>> Into<Description> for Spanned<T> {
    fn into(self) -> Description {
        Description::Source(self.map(|s| s.into()))
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

#[derive(Debug, Clone)]
pub enum ParseErrorReason {
    Eof {
        expected: &'static str,
        span: Span,
    },
    Mismatch {
        expected: &'static str,
        actual: Spanned<String>,
    },
    ArgumentError {
        command: Spanned<String>,
        error: ArgumentError,
    },
}

#[derive(Debug, Clone)]
pub struct ParseError {
    reason: ParseErrorReason,
}

impl ParseError {
    pub fn unexpected_eof(expected: &'static str, span: Span) -> ParseError {
        ParseError {
            reason: ParseErrorReason::Eof { expected, span },
        }
    }

    pub fn mismatch(expected: &'static str, actual: Spanned<impl Into<String>>) -> ParseError {
        let Spanned { span, item } = actual;

        ParseError {
            reason: ParseErrorReason::Mismatch {
                expected,
                actual: item.into().spanned(span),
            },
        }
    }

    pub fn argument_error(command: Spanned<impl Into<String>>, kind: ArgumentError) -> ParseError {
        ParseError {
            reason: ParseErrorReason::ArgumentError {
                command: command.item.into().spanned(command.span),
                error: kind,
            },
        }
    }
}

impl From<ParseError> for ShellError {
    fn from(error: ParseError) -> ShellError {
        match error.reason {
            ParseErrorReason::Eof { expected, span } => ShellError::unexpected_eof(expected, span),
            ParseErrorReason::Mismatch { actual, expected } => {
                ShellError::type_error(expected, actual.clone())
            }
            ParseErrorReason::ArgumentError { command, error } => {
                ShellError::argument_error(command, error)
            }
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Ord, Hash, PartialOrd, Serialize, Deserialize)]
pub enum ArgumentError {
    MissingMandatoryFlag(String),
    MissingMandatoryPositional(String),
    MissingValueForName(String),
    InvalidExternalWord,
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Clone, Serialize, Deserialize, Hash)]
pub struct ShellError {
    error: ProximateShellError,
    cause: Option<Box<ProximateShellError>>,
}

impl FormatDebug for ShellError {
    fn fmt_debug(&self, f: &mut DebugFormatter, source: &str) -> fmt::Result {
        self.error.fmt_debug(f, source)
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
            subpath: subpath.into(),
            expr: expr.into(),
        }
        .start()
    }

    pub fn invalid_integer_index(
        subpath: Spanned<impl Into<String>>,
        integer: impl Into<Span>,
    ) -> ShellError {
        ProximateShellError::InvalidIntegerIndex {
            subpath: subpath.into(),
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

    pub(crate) fn unexpected_eof(expected: impl Into<String>, span: impl Into<Span>) -> ShellError {
        ProximateShellError::UnexpectedEof {
            expected: expected.into(),
            span: span.into(),
        }
        .start()
    }

    pub(crate) fn range_error(
        expected: impl Into<ExpectedRange>,
        actual: &Tagged<impl fmt::Debug>,
        operation: impl Into<String>,
    ) -> ShellError {
        ProximateShellError::RangeError {
            kind: expected.into(),
            actual_kind: format!("{:?}", actual.item).spanned(actual.span()),
            operation: operation.into(),
        }
        .start()
    }

    pub(crate) fn syntax_error(problem: Spanned<impl Into<String>>) -> ShellError {
        ProximateShellError::SyntaxError {
            problem: problem.map(|p| p.into()),
        }
        .start()
    }

    pub(crate) fn coerce_error(
        left: Spanned<impl Into<String>>,
        right: Spanned<impl Into<String>>,
    ) -> ShellError {
        ProximateShellError::CoerceError {
            left: left.map(|l| l.into()),
            right: right.map(|r| r.into()),
        }
        .start()
    }

    pub(crate) fn missing_value(span: Option<Span>, reason: impl Into<String>) -> ShellError {
        ProximateShellError::MissingValue {
            span,
            reason: reason.into(),
        }
        .start()
    }

    pub(crate) fn argument_error(
        command: Spanned<impl Into<String>>,
        kind: ArgumentError,
    ) -> ShellError {
        ProximateShellError::ArgumentError {
            command: command.map(|c| c.into()),
            error: kind,
        }
        .start()
    }

    pub(crate) fn parse_error(
        error: nom::Err<(
            nom_locate::LocatedSpanEx<&str, TracableContext>,
            nom::error::ErrorKind,
        )>,
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
            } => match error {
                ArgumentError::InvalidExternalWord => Diagnostic::new(
                    Severity::Error,
                    format!("Invalid bare word for Nu command (did you intend to invoke an external command?)"))
                .with_label(Label::new_primary(command.span)),
                ArgumentError::MissingMandatoryFlag(name) => Diagnostic::new(
                    Severity::Error,
                    format!(
                        "{} requires {}{}",
                        Color::Cyan.paint(&command.item),
                        Color::Black.bold().paint("--"),
                        Color::Black.bold().paint(name)
                    ),
                )
                .with_label(Label::new_primary(command.span)),
                ArgumentError::MissingMandatoryPositional(name) => Diagnostic::new(
                    Severity::Error,
                    format!(
                        "{} requires {} parameter",
                        Color::Cyan.paint(&command.item),
                        Color::Green.bold().paint(name.clone())
                    ),
                )
                .with_label(
                    Label::new_primary(command.span).with_message(format!("requires {} parameter", name)),
                ),
                ArgumentError::MissingValueForName(name) => Diagnostic::new(
                    Severity::Error,
                    format!(
                        "{} is missing value for flag {}{}",
                        Color::Cyan.paint(&command.item),
                        Color::Black.bold().paint("--"),
                        Color::Black.bold().paint(name)
                    ),
                )
                .with_label(Label::new_primary(command.span)),
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
                actual:
                    Spanned {
                        item: None,
                        span
                    },
            } => Diagnostic::new(Severity::Error, "Type Error")
                .with_label(Label::new_primary(span).with_message(expected)),

            ProximateShellError::UnexpectedEof {
                expected, span
            } => Diagnostic::new(Severity::Error, format!("Unexpected end of input"))
                .with_label(Label::new_primary(span).with_message(format!("Expected {}", expected))),

            ProximateShellError::RangeError {
                kind,
                operation,
                actual_kind:
                    Spanned {
                        item,
                        span
                    },
            } => Diagnostic::new(Severity::Error, "Range Error").with_label(
                Label::new_primary(span).with_message(format!(
                    "Expected to convert {} to {} while {}, but it was out of range",
                    item,
                    kind.desc(),
                    operation
                )),
            ),

            ProximateShellError::SyntaxError {
                problem:
                    Spanned {
                        span,
                        item
                    },
            } => Diagnostic::new(Severity::Error, "Syntax Error")
                .with_label(Label::new_primary(span).with_message(item)),

            ProximateShellError::MissingProperty { subpath, expr, .. } => {
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

            ProximateShellError::InvalidIntegerIndex { subpath,integer } => {
                let subpath = subpath.into_label();

                let mut diag = Diagnostic::new(Severity::Error, "Invalid integer property");

                match subpath {
                    Ok(label) => diag = diag.with_label(label),
                    Err(ty) => diag.message = format!("Invalid integer property (for {})", ty)
                }

                diag = diag.with_label(Label::new_secondary(integer).with_message("integer"));

                diag
            }

            ProximateShellError::Diagnostic(diag) => diag.diagnostic,
            ProximateShellError::CoerceError { left, right } => {
                Diagnostic::new(Severity::Error, "Coercion error")
                    .with_label(Label::new_primary(left.span).with_message(left.item))
                    .with_label(Label::new_secondary(right.span).with_message(right.item))
            }

            ProximateShellError::UntaggedRuntimeError { reason } => Diagnostic::new(Severity::Error, format!("Error: {}", reason))
        }
    }

    pub fn labeled_error(
        msg: impl Into<String>,
        label: impl Into<String>,
        tag: impl Into<Tag>,
    ) -> ShellError {
        ShellError::diagnostic(
            Diagnostic::new(Severity::Error, msg.into())
                .with_label(Label::new_primary(tag.into().span).with_message(label.into())),
        )
    }

    pub fn labeled_error_with_secondary(
        msg: impl Into<String>,
        primary_label: impl Into<String>,
        primary_span: impl Into<Tag>,
        secondary_label: impl Into<String>,
        secondary_span: impl Into<Tag>,
    ) -> ShellError {
        ShellError::diagnostic(
            Diagnostic::new_error(msg.into())
                .with_label(
                    Label::new_primary(primary_span.into().span).with_message(primary_label.into()),
                )
                .with_label(
                    Label::new_secondary(secondary_span.into().span)
                        .with_message(secondary_label.into()),
                ),
        )
    }

    // pub fn string(title: impl Into<String>) -> ShellError {
    //     ProximateShellError::String(StringError::new(title.into(), String::new())).start()
    // }
    //
    // pub(crate) fn unreachable(title: impl Into<String>) -> ShellError {
    //     ShellError::untagged_runtime_error(&format!("BUG: Unreachable: {}", title.into()))
    // }

    pub(crate) fn unimplemented(title: impl Into<String>) -> ShellError {
        ShellError::untagged_runtime_error(&format!("Unimplemented: {}", title.into()))
    }

    pub(crate) fn unexpected(title: impl Into<String>) -> ShellError {
        ShellError::untagged_runtime_error(&format!("Unexpected: {}", title.into()))
    }
}

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

impl From<Range<usize>> for ExpectedRange {
    fn from(range: Range<usize>) -> Self {
        ExpectedRange::Range {
            start: range.start,
            end: range.end,
        }
    }
}

impl ExpectedRange {
    fn desc(&self) -> String {
        match self {
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
            ExpectedRange::Range { start, end } => return format!("{} to {}", start, end),
        }
        .to_string()
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
        subpath: Description,
        expr: Description,
    },
    InvalidIntegerIndex {
        subpath: Description,
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
}

impl ProximateShellError {
    fn start(self) -> ShellError {
        ShellError {
            cause: None,
            error: self,
        }
    }

    // pub(crate) fn tag(&self) -> Option<Tag> {
    //     Some(match self {
    //         ProximateShellError::SyntaxError { problem } => problem.tag(),
    //         ProximateShellError::UnexpectedEof { tag, .. } => tag.clone(),
    //         ProximateShellError::InvalidCommand { command } => command.clone(),
    //         ProximateShellError::TypeError { actual, .. } => actual.tag.clone(),
    //         ProximateShellError::MissingProperty { tag, .. } => tag.clone(),
    //         ProximateShellError::MissingValue { tag, .. } => return tag.clone(),
    //         ProximateShellError::ArgumentError { tag, .. } => tag.clone(),
    //         ProximateShellError::RangeError { actual_kind, .. } => actual_kind.tag.clone(),
    //         ProximateShellError::InvalidIntegerIndex { integer, .. } => integer.into(),
    //         ProximateShellError::Diagnostic(..) => return None,
    //         ProximateShellError::UntaggedRuntimeError { .. } => return None,
    //         ProximateShellError::CoerceError { left, right } => left.tag.until(&right.tag),
    //     })
    // }
}

impl FormatDebug for ProximateShellError {
    fn fmt_debug(&self, f: &mut DebugFormatter, _source: &str) -> fmt::Result {
        // TODO: Custom debug for inner spans
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellDiagnostic {
    pub(crate) diagnostic: Diagnostic<Span>,
}

impl std::hash::Hash for ShellDiagnostic {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.diagnostic.severity.hash(state);
        self.diagnostic.code.hash(state);
        self.diagnostic.message.hash(state);

        for label in &self.diagnostic.labels {
            label.span.hash(state);
            label.message.hash(state);
            match label.style {
                language_reporting::LabelStyle::Primary => 0.hash(state),
                language_reporting::LabelStyle::Secondary => 1.hash(state),
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

impl std::fmt::Display for ShellError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self.error {
            ProximateShellError::MissingValue { .. } => write!(f, "MissingValue"),
            ProximateShellError::TypeError { .. } => write!(f, "TypeError"),
            ProximateShellError::UnexpectedEof { .. } => write!(f, "UnexpectedEof"),
            ProximateShellError::RangeError { .. } => write!(f, "RangeError"),
            ProximateShellError::InvalidIntegerIndex { .. } => write!(f, "InvalidIntegerIndex"),
            ProximateShellError::SyntaxError { .. } => write!(f, "SyntaxError"),
            ProximateShellError::MissingProperty { .. } => write!(f, "MissingProperty"),
            ProximateShellError::ArgumentError { .. } => write!(f, "ArgumentError"),
            ProximateShellError::Diagnostic(_) => write!(f, "<diagnostic>"),
            ProximateShellError::CoerceError { .. } => write!(f, "CoerceError"),
            ProximateShellError::UntaggedRuntimeError { .. } => write!(f, "UntaggedRuntimeError"),
        }
    }
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

impl std::convert::From<subprocess::PopenError> for ShellError {
    fn from(input: subprocess::PopenError) -> ShellError {
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

        impl CoerceInto<$ty> for Tagged<BigInt> {
            fn coerce_into(self, operation: impl Into<String>) -> Result<$ty, ShellError> {
                match self.$op() {
                    Some(v) => Ok(v),
                    None => Err(ShellError::range_error(
                        $ty::to_expected_range(),
                        &self,
                        operation.into(),
                    )),
                }
            }
        }

        impl CoerceInto<$ty> for Tagged<&BigInt> {
            fn coerce_into(self, operation: impl Into<String>) -> Result<$ty, ShellError> {
                match self.$op() {
                    Some(v) => Ok(v),
                    None => Err(ShellError::range_error(
                        $ty::to_expected_range(),
                        &self,
                        operation.into(),
                    )),
                }
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

        impl CoerceInto<$ty> for Tagged<BigDecimal> {
            fn coerce_into(self, operation: impl Into<String>) -> Result<$ty, ShellError> {
                match self.$op() {
                    Some(v) => Ok(v),
                    None => Err(ShellError::range_error(
                        $ty::to_expected_range(),
                        &self,
                        operation.into(),
                    )),
                }
            }
        }

        impl CoerceInto<$ty> for Tagged<&BigDecimal> {
            fn coerce_into(self, operation: impl Into<String>) -> Result<$ty, ShellError> {
                match self.$op() {
                    Some(v) => Ok(v),
                    None => Err(ShellError::range_error(
                        $ty::to_expected_range(),
                        &self,
                        operation.into(),
                    )),
                }
            }
        }
    };
}

ranged_decimal!(f32 -> to_f32 -> F32);
ranged_decimal!(f64 -> to_f64 -> F64);
