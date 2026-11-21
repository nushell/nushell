//! Conversions between [`DateTime`] and `chrono::DateTime<FixedOffset>`, for plugins that still
//! use chrono. Enabled by nu-protocol's `chrono` feature.

use super::{DateParseError, DateTime};
use crate::{FromValue, IntoValue, ShellError, Span, Type, Value};
use jiff::tz::Offset;

/// chrono reaches year 262142, past the year 9999 where nushell's dates end; such a date is
/// [`DateParseError::OutOfRange`]. chrono keeps a leap second (`23:59:60`) as a second's worth
/// of extra nanoseconds, which reads as second 59 here.
impl TryFrom<chrono::DateTime<chrono::FixedOffset>> for DateTime {
    type Error = DateParseError;

    fn try_from(value: chrono::DateTime<chrono::FixedOffset>) -> Result<Self, Self::Error> {
        let offset = Offset::from_seconds(value.offset().local_minus_utc()).unwrap_or(Offset::UTC);
        DateTime::from_unix_timestamp(value.timestamp(), value.timestamp_subsec_nanos(), offset)
            .ok_or(DateParseError::OutOfRange)
    }
}

/// chrono's offsets stop short of a day; a longer offset is clamped to 23:59:59.
impl From<DateTime> for chrono::DateTime<chrono::FixedOffset> {
    fn from(value: DateTime) -> Self {
        let seconds = value.offset().seconds().clamp(-86_399, 86_399);
        let offset = chrono::FixedOffset::east_opt(seconds).unwrap_or(chrono::Utc.fix());
        chrono::DateTime::from_timestamp(value.seconds, value.nanos)
            .unwrap_or_default()
            .with_timezone(&offset)
    }
}

use chrono::Offset as _;

impl FromValue for chrono::DateTime<chrono::FixedOffset> {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        DateTime::from_value(v).map(Into::into)
    }

    fn expected_type() -> Type {
        Type::Date
    }
}

/// A date past year 9999 is an error value.
impl IntoValue for chrono::DateTime<chrono::FixedOffset> {
    fn into_value(self, span: Span) -> Value {
        match DateTime::try_from(self) {
            Ok(date) => Value::date(date, span),
            Err(_) => Value::error(
                ShellError::CantConvert {
                    to_type: "date".into(),
                    from_type: format!("chrono date {}", self.to_rfc3339()),
                    span,
                    help: Some("nushell dates run from year -9999 to 9999".into()),
                },
                span,
            ),
        }
    }
}
