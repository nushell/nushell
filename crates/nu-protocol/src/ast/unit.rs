use crate::{Filesize, FilesizeUnit, IntoValue, ShellError, Span, Value};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

pub const SUPPORTED_DURATION_UNITS: [&str; 9] =
    ["ns", "us", "µs", "ms", "sec", "min", "hr", "day", "wk"];

/// The error returned when failing to parse a [`Unit`].
///
/// This occurs when the string being parsed does not exactly match the name of one of the
/// enum cases in [`Unit`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, Error)]
pub struct ParseUnitError(());

impl fmt::Display for ParseUnitError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "invalid file size or duration unit")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Unit {
    Filesize(FilesizeUnit),

    // Duration units
    Nanosecond,
    Microsecond,
    Millisecond,
    Second,
    Minute,
    Hour,
    Day,
    Week,
}

// TODO: something like `Filesize::from_unit` in the future?
fn duration_mul_and_check(size: i64, factor: i64, span: Span) -> Result<Value, ShellError> {
    match size.checked_mul(factor) {
        Some(val) => Ok(Value::duration(val, span)),
        None => Err(ShellError::GenericError {
            error: "duration too large".into(),
            msg: "duration too large".into(),
            span: Some(span),
            help: None,
            inner: vec![],
        }),
    }
}

impl Unit {
    pub fn build_value(self, size: i64, span: Span) -> Result<Value, ShellError> {
        match self {
            Unit::Filesize(unit) => {
                if let Some(filesize) = Filesize::from_unit(size, unit) {
                    Ok(filesize.into_value(span))
                } else {
                    Err(ShellError::GenericError {
                        error: "filesize too large".into(),
                        msg: "filesize too large".into(),
                        span: Some(span),
                        help: None,
                        inner: vec![],
                    })
                }
            }
            Unit::Nanosecond => Ok(Value::duration(size, span)),
            Unit::Microsecond => duration_mul_and_check(size, 1000, span),
            Unit::Millisecond => duration_mul_and_check(size, 1000 * 1000, span),
            Unit::Second => duration_mul_and_check(size, 1000 * 1000 * 1000, span),
            Unit::Minute => duration_mul_and_check(size, 1000 * 1000 * 1000 * 60, span),
            Unit::Hour => duration_mul_and_check(size, 1000 * 1000 * 1000 * 60 * 60, span),
            Unit::Day => duration_mul_and_check(size, 1000 * 1000 * 1000 * 60 * 60 * 24, span),
            Unit::Week => duration_mul_and_check(size, 1000 * 1000 * 1000 * 60 * 60 * 24 * 7, span),
        }
    }

    /// Returns the symbol [`str`] for a [`Unit`].
    ///
    /// The returned string is the same exact string needed for a successful call to
    /// [`parse`](str::parse) for a [`Unit`].
    ///
    /// # Examples
    /// ```
    /// # use nu_protocol::{Unit, FilesizeUnit};
    /// assert_eq!(Unit::Nanosecond.as_str(), "ns");
    /// assert_eq!(Unit::Filesize(FilesizeUnit::B).as_str(), "B");
    /// assert_eq!(Unit::Second.as_str().parse(), Ok(Unit::Second));
    /// assert_eq!(Unit::Filesize(FilesizeUnit::KB).as_str().parse(), Ok(Unit::Filesize(FilesizeUnit::KB)));
    /// ```
    pub const fn as_str(&self) -> &'static str {
        match self {
            Unit::Filesize(u) => u.as_str(),
            Unit::Nanosecond => "ns",
            Unit::Microsecond => "us",
            Unit::Millisecond => "ms",
            Unit::Second => "sec",
            Unit::Minute => "min",
            Unit::Hour => "hr",
            Unit::Day => "day",
            Unit::Week => "wk",
        }
    }
}

impl FromStr for Unit {
    type Err = ParseUnitError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(filesize_unit) = FilesizeUnit::from_str(s) {
            return Ok(Unit::Filesize(filesize_unit));
        };

        match s {
            "ns" => Ok(Unit::Nanosecond),
            "us" | "µs" => Ok(Unit::Microsecond),
            "ms" => Ok(Unit::Millisecond),
            "sec" => Ok(Unit::Second),
            "min" => Ok(Unit::Minute),
            "hr" => Ok(Unit::Hour),
            "day" => Ok(Unit::Day),
            "wk" => Ok(Unit::Week),
            _ => Err(ParseUnitError(())),
        }
    }
}
