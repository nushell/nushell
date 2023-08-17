use crate::{ShellError, Span, SpannedValue};
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
    pub fn to_value(&self, size: i64, span: Span) -> SpannedValue {
        match self {
            Unit::Byte => SpannedValue::Filesize { val: size, span },
            Unit::Kilobyte => SpannedValue::Filesize {
                val: size * 1000,
                span,
            },
            Unit::Megabyte => SpannedValue::Filesize {
                val: size * 1000 * 1000,
                span,
            },
            Unit::Gigabyte => SpannedValue::Filesize {
                val: size * 1000 * 1000 * 1000,
                span,
            },
            Unit::Terabyte => SpannedValue::Filesize {
                val: size * 1000 * 1000 * 1000 * 1000,
                span,
            },
            Unit::Petabyte => SpannedValue::Filesize {
                val: size * 1000 * 1000 * 1000 * 1000 * 1000,
                span,
            },
            Unit::Exabyte => SpannedValue::Filesize {
                val: size * 1000 * 1000 * 1000 * 1000 * 1000 * 1000,
                span,
            },

            Unit::Kibibyte => SpannedValue::Filesize {
                val: size * 1024,
                span,
            },
            Unit::Mebibyte => SpannedValue::Filesize {
                val: size * 1024 * 1024,
                span,
            },
            Unit::Gibibyte => SpannedValue::Filesize {
                val: size * 1024 * 1024 * 1024,
                span,
            },
            Unit::Tebibyte => SpannedValue::Filesize {
                val: size * 1024 * 1024 * 1024 * 1024,
                span,
            },
            Unit::Pebibyte => SpannedValue::Filesize {
                val: size * 1024 * 1024 * 1024 * 1024 * 1024,
                span,
            },
            Unit::Exbibyte => SpannedValue::Filesize {
                val: size * 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
                span,
            },

            Unit::Nanosecond => SpannedValue::Duration { val: size, span },
            Unit::Microsecond => SpannedValue::Duration {
                val: size * 1000,
                span,
            },
            Unit::Millisecond => SpannedValue::Duration {
                val: size * 1000 * 1000,
                span,
            },
            Unit::Second => SpannedValue::Duration {
                val: size * 1000 * 1000 * 1000,
                span,
            },
            Unit::Minute => match size.checked_mul(1000 * 1000 * 1000 * 60) {
                Some(val) => SpannedValue::Duration { val, span },
                None => SpannedValue::Error {
                    error: Box::new(ShellError::GenericError(
                        "duration too large".into(),
                        "duration too large".into(),
                        Some(span),
                        None,
                        Vec::new(),
                    )),
                    span,
                },
            },
            Unit::Hour => match size.checked_mul(1000 * 1000 * 1000 * 60 * 60) {
                Some(val) => SpannedValue::Duration { val, span },
                None => SpannedValue::Error {
                    error: Box::new(ShellError::GenericError(
                        "duration too large".into(),
                        "duration too large".into(),
                        Some(span),
                        None,
                        Vec::new(),
                    )),
                    span,
                },
            },
            Unit::Day => match size.checked_mul(1000 * 1000 * 1000 * 60 * 60 * 24) {
                Some(val) => SpannedValue::Duration { val, span },
                None => SpannedValue::Error {
                    error: Box::new(ShellError::GenericError(
                        "duration too large".into(),
                        "duration too large".into(),
                        Some(span),
                        None,
                        Vec::new(),
                    )),
                    span,
                },
            },
            Unit::Week => match size.checked_mul(1000 * 1000 * 1000 * 60 * 60 * 24 * 7) {
                Some(val) => SpannedValue::Duration { val, span },
                None => SpannedValue::Error {
                    error: Box::new(ShellError::GenericError(
                        "duration too large".into(),
                        "duration too large".into(),
                        Some(span),
                        None,
                        Vec::new(),
                    )),
                    span,
                },
            },
        }
    }
}
