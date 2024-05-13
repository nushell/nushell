use crate::{ShellError, Span, Value};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilesizeUnit {
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
}

impl FilesizeUnit {
    pub const fn is_metric(&self) -> bool {
        match self {
            FilesizeUnit::Byte
            | FilesizeUnit::Kilobyte
            | FilesizeUnit::Megabyte
            | FilesizeUnit::Gigabyte
            | FilesizeUnit::Terabyte
            | FilesizeUnit::Petabyte
            | FilesizeUnit::Exabyte => true,
            FilesizeUnit::Kibibyte
            | FilesizeUnit::Mebibyte
            | FilesizeUnit::Gibibyte
            | FilesizeUnit::Tebibyte
            | FilesizeUnit::Pebibyte
            | FilesizeUnit::Exbibyte => false,
        }
    }

    pub const fn as_str(&self) -> &'static str {
        match self {
            FilesizeUnit::Byte => "B",
            FilesizeUnit::Kilobyte => "KB",
            FilesizeUnit::Megabyte => "MB",
            FilesizeUnit::Gigabyte => "GB",
            FilesizeUnit::Terabyte => "TB",
            FilesizeUnit::Petabyte => "PB",
            FilesizeUnit::Exabyte => "EB",
            FilesizeUnit::Kibibyte => "KiB",
            FilesizeUnit::Mebibyte => "MiB",
            FilesizeUnit::Gibibyte => "GiB",
            FilesizeUnit::Tebibyte => "TiB",
            FilesizeUnit::Pebibyte => "PiB",
            FilesizeUnit::Exbibyte => "EiB",
        }
    }
}

impl Display for FilesizeUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<FilesizeUnit> for byte_unit::Unit {
    fn from(unit: FilesizeUnit) -> Self {
        match unit {
            FilesizeUnit::Byte => byte_unit::Unit::B,
            FilesizeUnit::Kilobyte => byte_unit::Unit::KB,
            FilesizeUnit::Megabyte => byte_unit::Unit::MB,
            FilesizeUnit::Gigabyte => byte_unit::Unit::GB,
            FilesizeUnit::Terabyte => byte_unit::Unit::TB,
            FilesizeUnit::Petabyte => byte_unit::Unit::PB,
            FilesizeUnit::Exabyte => byte_unit::Unit::EB,
            FilesizeUnit::Kibibyte => byte_unit::Unit::KiB,
            FilesizeUnit::Mebibyte => byte_unit::Unit::MiB,
            FilesizeUnit::Gibibyte => byte_unit::Unit::GiB,
            FilesizeUnit::Tebibyte => byte_unit::Unit::TiB,
            FilesizeUnit::Pebibyte => byte_unit::Unit::PiB,
            FilesizeUnit::Exbibyte => byte_unit::Unit::EiB,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DurationUnit {
    Nanosecond,
    Microsecond,
    Millisecond,
    Second,
    Minute,
    Hour,
    Day,
    Week,
}

impl DurationUnit {
    pub const fn as_str(&self) -> &'static str {
        match self {
            DurationUnit::Nanosecond => "ns",
            DurationUnit::Microsecond => "µs",
            DurationUnit::Millisecond => "ms",
            DurationUnit::Second => "sec",
            DurationUnit::Minute => "min",
            DurationUnit::Hour => "hr",
            DurationUnit::Day => "day",
            DurationUnit::Week => "wk",
        }
    }
}

impl Display for DurationUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Unit {
    Filesize(FilesizeUnit),
    Duration(DurationUnit),
}

impl Unit {
    pub fn build_value(self, size: i64, span: Span) -> Result<Value, ShellError> {
        match self {
            Unit::Filesize(unit) => match unit {
                FilesizeUnit::Byte => Ok(Value::filesize(size, span)),
                FilesizeUnit::Kilobyte => Ok(Value::filesize(size * 1000, span)),
                FilesizeUnit::Megabyte => Ok(Value::filesize(size * 1000 * 1000, span)),
                FilesizeUnit::Gigabyte => Ok(Value::filesize(size * 1000 * 1000 * 1000, span)),
                FilesizeUnit::Terabyte => {
                    Ok(Value::filesize(size * 1000 * 1000 * 1000 * 1000, span))
                }
                FilesizeUnit::Petabyte => Ok(Value::filesize(
                    size * 1000 * 1000 * 1000 * 1000 * 1000,
                    span,
                )),
                FilesizeUnit::Exabyte => Ok(Value::filesize(
                    size * 1000 * 1000 * 1000 * 1000 * 1000 * 1000,
                    span,
                )),

                FilesizeUnit::Kibibyte => Ok(Value::filesize(size * 1024, span)),
                FilesizeUnit::Mebibyte => Ok(Value::filesize(size * 1024 * 1024, span)),
                FilesizeUnit::Gibibyte => Ok(Value::filesize(size * 1024 * 1024 * 1024, span)),
                FilesizeUnit::Tebibyte => {
                    Ok(Value::filesize(size * 1024 * 1024 * 1024 * 1024, span))
                }
                FilesizeUnit::Pebibyte => Ok(Value::filesize(
                    size * 1024 * 1024 * 1024 * 1024 * 1024,
                    span,
                )),
                FilesizeUnit::Exbibyte => Ok(Value::filesize(
                    size * 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
                    span,
                )),
            },
            Unit::Duration(unit) => match unit {
                DurationUnit::Nanosecond => Ok(Value::duration(size, span)),
                DurationUnit::Microsecond => Ok(Value::duration(size * 1000, span)),
                DurationUnit::Millisecond => Ok(Value::duration(size * 1000 * 1000, span)),
                DurationUnit::Second => Ok(Value::duration(size * 1000 * 1000 * 1000, span)),
                DurationUnit::Minute => match size.checked_mul(1000 * 1000 * 1000 * 60) {
                    Some(val) => Ok(Value::duration(val, span)),
                    None => Err(ShellError::GenericError {
                        error: "duration too large".into(),
                        msg: "duration too large".into(),
                        span: Some(span),
                        help: None,
                        inner: vec![],
                    }),
                },
                DurationUnit::Hour => match size.checked_mul(1000 * 1000 * 1000 * 60 * 60) {
                    Some(val) => Ok(Value::duration(val, span)),
                    None => Err(ShellError::GenericError {
                        error: "duration too large".into(),
                        msg: "duration too large".into(),
                        span: Some(span),
                        help: None,
                        inner: vec![],
                    }),
                },
                DurationUnit::Day => match size.checked_mul(1000 * 1000 * 1000 * 60 * 60 * 24) {
                    Some(val) => Ok(Value::duration(val, span)),
                    None => Err(ShellError::GenericError {
                        error: "duration too large".into(),
                        msg: "duration too large".into(),
                        span: Some(span),
                        help: None,
                        inner: vec![],
                    }),
                },
                DurationUnit::Week => match size.checked_mul(1000 * 1000 * 1000 * 60 * 60 * 24 * 7)
                {
                    Some(val) => Ok(Value::duration(val, span)),
                    None => Err(ShellError::GenericError {
                        error: "duration too large".into(),
                        msg: "duration too large".into(),
                        span: Some(span),
                        help: None,
                        inner: vec![],
                    }),
                },
            },
        }
    }
}

impl From<FilesizeUnit> for Unit {
    fn from(unit: FilesizeUnit) -> Self {
        Self::Filesize(unit)
    }
}

impl From<DurationUnit> for Unit {
    fn from(unit: DurationUnit) -> Self {
        Self::Duration(unit)
    }
}
