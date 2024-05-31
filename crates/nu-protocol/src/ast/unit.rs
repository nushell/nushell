use crate::{ShellError, Span, Value};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Unit {
    // Filesize units: metric
    Byte,
    Kilobyte,
    Megabyte,
    Gigabyte,
    Terabyte,
    Petabyte,
    Exabyte,

    // Filesize units: ISO/IEC 80000
    Kibibyte,
    Mebibyte,
    Gibibyte,
    Tebibyte,
    Pebibyte,
    Exbibyte,

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

impl Unit {
    pub fn build_value(self, size: i64, span: Span) -> Result<Value, ShellError> {
        match self {
            Unit::Byte => Ok(Value::filesize(size, span)),
            Unit::Kilobyte => Ok(Value::filesize(size * 1000, span)),
            Unit::Megabyte => Ok(Value::filesize(size * 1000 * 1000, span)),
            Unit::Gigabyte => Ok(Value::filesize(size * 1000 * 1000 * 1000, span)),
            Unit::Terabyte => Ok(Value::filesize(size * 1000 * 1000 * 1000 * 1000, span)),
            Unit::Petabyte => Ok(Value::filesize(
                size * 1000 * 1000 * 1000 * 1000 * 1000,
                span,
            )),
            Unit::Exabyte => Ok(Value::filesize(
                size * 1000 * 1000 * 1000 * 1000 * 1000 * 1000,
                span,
            )),

            Unit::Kibibyte => Ok(Value::filesize(size * 1024, span)),
            Unit::Mebibyte => Ok(Value::filesize(size * 1024 * 1024, span)),
            Unit::Gibibyte => Ok(Value::filesize(size * 1024 * 1024 * 1024, span)),
            Unit::Tebibyte => Ok(Value::filesize(size * 1024 * 1024 * 1024 * 1024, span)),
            Unit::Pebibyte => Ok(Value::filesize(
                size * 1024 * 1024 * 1024 * 1024 * 1024,
                span,
            )),
            Unit::Exbibyte => Ok(Value::filesize(
                size * 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
                span,
            )),

            Unit::Nanosecond => Ok(Value::duration(size, span)),
            Unit::Microsecond => Ok(Value::duration(size * 1000, span)),
            Unit::Millisecond => Ok(Value::duration(size * 1000 * 1000, span)),
            Unit::Second => Ok(Value::duration(size * 1000 * 1000 * 1000, span)),
            Unit::Minute => match size.checked_mul(1000 * 1000 * 1000 * 60) {
                Some(val) => Ok(Value::duration(val, span)),
                None => Err(ShellError::GenericError {
                    error: "duration too large".into(),
                    msg: "duration too large".into(),
                    span: Some(span),
                    help: None,
                    inner: vec![],
                }),
            },
            Unit::Hour => match size.checked_mul(1000 * 1000 * 1000 * 60 * 60) {
                Some(val) => Ok(Value::duration(val, span)),
                None => Err(ShellError::GenericError {
                    error: "duration too large".into(),
                    msg: "duration too large".into(),
                    span: Some(span),
                    help: None,
                    inner: vec![],
                }),
            },
            Unit::Day => match size.checked_mul(1000 * 1000 * 1000 * 60 * 60 * 24) {
                Some(val) => Ok(Value::duration(val, span)),
                None => Err(ShellError::GenericError {
                    error: "duration too large".into(),
                    msg: "duration too large".into(),
                    span: Some(span),
                    help: None,
                    inner: vec![],
                }),
            },
            Unit::Week => match size.checked_mul(1000 * 1000 * 1000 * 60 * 60 * 24 * 7) {
                Some(val) => Ok(Value::duration(val, span)),
                None => Err(ShellError::GenericError {
                    error: "duration too large".into(),
                    msg: "duration too large".into(),
                    span: Some(span),
                    help: None,
                    inner: vec![],
                }),
            },
        }
    }
}
