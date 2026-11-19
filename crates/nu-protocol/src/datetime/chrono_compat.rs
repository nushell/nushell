//! Conversions between [`DateTime`] and `chrono::DateTime<FixedOffset>`, for code that still
//! uses chrono.

use super::{DateParseError, DateTime};
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
