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
    Zettabyte,

    // Filesize units: ISO/IEC 80000
    Kibibyte,
    Mebibyte,
    Gibibyte,
    Tebibyte,
    Pebibyte,
    Exbibyte,
    Zebibyte,

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
    pub fn to_value(&self, size: i64, span: Span) -> Value {
        match self {
            Unit::Byte => Value::Filesize { val: size, span },
            Unit::Kilobyte => Value::Filesize {
                val: size * 1000,
                span,
            },
            Unit::Megabyte => Value::Filesize {
                val: size * 1000 * 1000,
                span,
            },
            Unit::Gigabyte => Value::Filesize {
                val: size * 1000 * 1000 * 1000,
                span,
            },
            Unit::Terabyte => Value::Filesize {
                val: size * 1000 * 1000 * 1000 * 1000,
                span,
            },
            Unit::Petabyte => Value::Filesize {
                val: size * 1000 * 1000 * 1000 * 1000 * 1000,
                span,
            },
            Unit::Exabyte => Value::Filesize {
                val: size * 1000 * 1000 * 1000 * 1000 * 1000 * 1000,
                span,
            },
            Unit::Zettabyte => Value::Filesize {
                val: size * 1000 * 1000 * 1000 * 1000 * 1000 * 1000 * 1000,
                span,
            },

            Unit::Kibibyte => Value::Filesize {
                val: size * 1024,
                span,
            },
            Unit::Mebibyte => Value::Filesize {
                val: size * 1024 * 1024,
                span,
            },
            Unit::Gibibyte => Value::Filesize {
                val: size * 1024 * 1024 * 1024,
                span,
            },
            Unit::Tebibyte => Value::Filesize {
                val: size * 1024 * 1024 * 1024 * 1024,
                span,
            },
            Unit::Pebibyte => Value::Filesize {
                val: size * 1024 * 1024 * 1024 * 1024 * 1024,
                span,
            },
            Unit::Exbibyte => Value::Filesize {
                val: size * 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
                span,
            },
            Unit::Zebibyte => Value::Filesize {
                val: size * 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
                span,
            },

            Unit::Nanosecond => Value::Duration { val: size, span },
            Unit::Microsecond => Value::Duration {
                val: size * 1000,
                span,
            },
            Unit::Millisecond => Value::Duration {
                val: size * 1000 * 1000,
                span,
            },
            Unit::Second => Value::Duration {
                val: size * 1000 * 1000 * 1000,
                span,
            },
            Unit::Minute => match size.checked_mul(1000 * 1000 * 1000 * 60) {
                Some(val) => Value::Duration { val, span },
                None => Value::Error {
                    error: Box::new(ShellError::GenericError(
                        "duration too large".into(),
                        "duration too large".into(),
                        Some(span),
                        None,
                        Vec::new(),
                    )),
                },
            },
            Unit::Hour => match size.checked_mul(1000 * 1000 * 1000 * 60 * 60) {
                Some(val) => Value::Duration { val, span },
                None => Value::Error {
                    error: Box::new(ShellError::GenericError(
                        "duration too large".into(),
                        "duration too large".into(),
                        Some(span),
                        None,
                        Vec::new(),
                    )),
                },
            },
            Unit::Day => match size.checked_mul(1000 * 1000 * 1000 * 60 * 60 * 24) {
                Some(val) => Value::Duration { val, span },
                None => Value::Error {
                    error: Box::new(ShellError::GenericError(
                        "duration too large".into(),
                        "duration too large".into(),
                        Some(span),
                        None,
                        Vec::new(),
                    )),
                },
            },
            Unit::Week => match size.checked_mul(1000 * 1000 * 1000 * 60 * 60 * 24 * 7) {
                Some(val) => Value::Duration { val, span },
                None => Value::Error {
                    error: Box::new(ShellError::GenericError(
                        "duration too large".into(),
                        "duration too large".into(),
                        Some(span),
                        None,
                        Vec::new(),
                    )),
                },
            },
        }
    }
}
