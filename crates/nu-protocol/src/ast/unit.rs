use crate::{Filesize, FilesizeUnit, IntoValue, ShellError, Span, Value};
use serde::{Deserialize, Serialize};

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
