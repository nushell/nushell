//! Conversions between [`DateTime`] and `chrono::DateTime<FixedOffset>`, for plugins that still
//! use chrono. Enabled by nu-protocol's `chrono` feature.

use super::{DateParseError, DateTime};
use crate::{FromValue, IntoValue, ShellError, Span, Type, Value};
use jiff::{Timestamp, tz::Offset};

/// Fails for dates outside jiff's range (years -9999 to 9999). A leap second becomes the last
/// nanosecond of its minute.
impl TryFrom<chrono::DateTime<chrono::FixedOffset>> for DateTime {
    type Error = DateParseError;

    fn try_from(value: chrono::DateTime<chrono::FixedOffset>) -> Result<Self, Self::Error> {
        // chrono floors seconds and keeps nanoseconds positive; jiff gives both one sign, so
        // the instant crosses as a single count of nanoseconds.
        let nanos = value.timestamp_subsec_nanos().min(999_999_999);
        let total = i128::from(value.timestamp()) * 1_000_000_000 + i128::from(nanos);
        let timestamp =
            Timestamp::from_nanosecond(total).map_err(|_| DateParseError::OutOfRange)?;
        let offset = Offset::from_seconds(value.offset().local_minus_utc())
            .map_err(|_| DateParseError::OutOfRange)?;
        Ok(Self::new(timestamp, offset))
    }
}

/// chrono's offsets stop short of a day; a longer jiff offset is clamped to 23:59:59.
impl From<DateTime> for chrono::DateTime<chrono::FixedOffset> {
    fn from(value: DateTime) -> Self {
        let seconds = value.offset().seconds().clamp(-86_399, 86_399);
        let offset = chrono::FixedOffset::east_opt(seconds).unwrap_or(chrono::Utc.fix());
        let nanos = value.unix_nanos();
        let (seconds, subsec) = (
            nanos.div_euclid(1_000_000_000),
            nanos.rem_euclid(1_000_000_000),
        );
        chrono::DateTime::from_timestamp(seconds as i64, subsec as u32)
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

/// A date outside jiff's years (-9999 to 9999) becomes an error value.
impl IntoValue for chrono::DateTime<chrono::FixedOffset> {
    fn into_value(self, span: Span) -> Value {
        match DateTime::try_from(self) {
            Ok(date) => Value::date(date, span),
            Err(_) => Value::error(
                ShellError::CantConvert {
                    to_type: "date".into(),
                    from_type: "chrono::DateTime<FixedOffset>".into(),
                    span,
                    help: Some("nushell dates cover the years -9999 to 9999".into()),
                },
                span,
            ),
        }
    }
}
