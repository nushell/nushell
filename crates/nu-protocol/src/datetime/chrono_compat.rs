//! Conversions between [`DateTime`] and `chrono::DateTime<FixedOffset>`, for plugins that still
//! use chrono. Enabled by nu-protocol's `chrono` feature.

use super::DateTime;
use crate::{FromValue, IntoValue, ShellError, Span, Type, Value};
use jiff::tz::Offset;

/// chrono's range is the same as nushell's, and a leap second stays one.
impl From<chrono::DateTime<chrono::FixedOffset>> for DateTime {
    fn from(value: chrono::DateTime<chrono::FixedOffset>) -> Self {
        let offset = Offset::from_seconds(value.offset().local_minus_utc()).unwrap_or(Offset::UTC);
        Self {
            seconds: value.timestamp(),
            nanos: value.timestamp_subsec_nanos(),
            offset,
        }
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

impl IntoValue for chrono::DateTime<chrono::FixedOffset> {
    fn into_value(self, span: Span) -> Value {
        Value::date(self.into(), span)
    }
}
