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
    pub fn to_value(&self, size: i64, span: Span) -> Result<Value, ShellError> {
        match self {
            Unit::Byte => Ok(Value::Filesize { val: size, span }),
            Unit::Kilobyte => Ok(Value::Filesize {
                val: size * 1000,
                span,
            }),
            Unit::Megabyte => Ok(Value::Filesize {
                val: size * 1000 * 1000,
                span,
            }),
            Unit::Gigabyte => Ok(Value::Filesize {
                val: size * 1000 * 1000 * 1000,
                span,
            }),
            Unit::Terabyte => Ok(Value::Filesize {
                val: size * 1000 * 1000 * 1000 * 1000,
                span,
            }),
            Unit::Petabyte => Ok(Value::Filesize {
                val: size * 1000 * 1000 * 1000 * 1000 * 1000,
                span,
            }),
            Unit::Exabyte => Ok(Value::Filesize {
                val: size * 1000 * 1000 * 1000 * 1000 * 1000 * 1000,
                span,
            }),

            Unit::Kibibyte => Ok(Value::Filesize {
                val: size * 1024,
                span,
            }),
            Unit::Mebibyte => Ok(Value::Filesize {
                val: size * 1024 * 1024,
                span,
            }),
            Unit::Gibibyte => Ok(Value::Filesize {
                val: size * 1024 * 1024 * 1024,
                span,
            }),
            Unit::Tebibyte => Ok(Value::Filesize {
                val: size * 1024 * 1024 * 1024 * 1024,
                span,
            }),
            Unit::Pebibyte => Ok(Value::Filesize {
                val: size * 1024 * 1024 * 1024 * 1024 * 1024,
                span,
            }),
            Unit::Exbibyte => Ok(Value::Filesize {
                val: size * 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
                span,
            }),

            Unit::Nanosecond => Ok(Value::Duration { val: size, span }),
            Unit::Microsecond => Ok(Value::Duration {
                val: size * 1000,
                span,
            }),
            Unit::Millisecond => Ok(Value::Duration {
                val: size * 1000 * 1000,
                span,
            }),
            Unit::Second => Ok(Value::Duration {
                val: size * 1000 * 1000 * 1000,
                span,
            }),
            Unit::Minute => match size.checked_mul(1000 * 1000 * 1000 * 60) {
                Some(val) => Ok(Value::Duration { val, span }),
                None => Err(ShellError::GenericError(
                    "duration too large".into(),
                    "duration too large".into(),
                    Some(span),
                    None,
                    Vec::new(),
                )),
            },
            Unit::Hour => match size.checked_mul(1000 * 1000 * 1000 * 60 * 60) {
                Some(val) => Ok(Value::Duration { val, span }),
                None => Err(ShellError::GenericError(
                    "duration too large".into(),
                    "duration too large".into(),
                    Some(span),
                    None,
                    Vec::new(),
                )),
            },
            Unit::Day => match size.checked_mul(1000 * 1000 * 1000 * 60 * 60 * 24) {
                Some(val) => Ok(Value::Duration { val, span }),
                None => Err(ShellError::GenericError(
                    "duration too large".into(),
                    "duration too large".into(),
                    Some(span),
                    None,
                    Vec::new(),
                )),
            },
            Unit::Week => match size.checked_mul(1000 * 1000 * 1000 * 60 * 60 * 24 * 7) {
                Some(val) => Ok(Value::Duration { val, span }),
                None => Err(ShellError::GenericError(
                    "duration too large".into(),
                    "duration too large".into(),
                    Some(span),
                    None,
                    Vec::new(),
                )),
            },
        }
    }
}
