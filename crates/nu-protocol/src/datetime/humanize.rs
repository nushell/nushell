//! Rough English text for a time span, such as `a day ago` or `in 3 weeks`.
//!
//! The wording and thresholds are those of the chrono-humanize crate's rough mode, which nushell
//! used to display dates.

use super::DateTime;
use std::fmt;

const MINUTE: i128 = 60;
const HOUR: i128 = MINUTE * 60;
const DAY: i128 = HOUR * 24;
const WEEK: i128 = DAY * 7;
const MONTH: i128 = DAY * 30;
const YEAR: i128 = DAY * 365;

/// A signed span of time, displayed as rough English: `now` within 10 seconds, otherwise
/// `in 2 hours` or `3 weeks ago`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HumanTime {
    nanos: i128,
}

impl HumanTime {
    /// A span of `nanos` nanoseconds; negative spans are in the past.
    pub const fn from_nanos(nanos: i128) -> Self {
        Self { nanos }
    }

    /// A span of `seconds` seconds; negative spans are in the past.
    pub const fn from_seconds(seconds: i64) -> Self {
        Self::from_nanos(seconds as i128 * 1_000_000_000)
    }

    /// The span from now to `date`, rounded to the second.
    ///
    /// Spans too long for 64-bit nanoseconds (about 292 years) count whole calendar years
    /// between the two dates, as nushell always has.
    pub fn from_now(date: DateTime) -> Self {
        let now = DateTime::now().with_offset(date.offset());
        let nanos = date.nanos_since(now);
        if i64::try_from(nanos).is_ok() {
            let seconds = (nanos as f64 / 1_000_000_000.0).round() as i64;
            Self::from_seconds(seconds)
        } else {
            let years = i64::from(date.year()) - i64::from(now.year());
            Self::from_seconds(years * 365 * DAY as i64)
        }
    }

    /// The span from now to `date`, not rounded.
    pub fn since_now(date: DateTime) -> Self {
        Self::from_nanos(date.nanos_since(DateTime::now_utc()))
    }
}

impl fmt::Display for HumanTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Whole seconds, truncated toward zero.
        let seconds = (self.nanos / 1_000_000_000).abs();
        let text = match seconds {
            n if n > 547 * DAY => plural(n / YEAR, "year"),
            n if n > 345 * DAY => "a year".into(),
            n if n > 45 * DAY => plural(n / MONTH, "month"),
            n if n > 29 * DAY => "a month".into(),
            n if n > 10 * DAY + 12 * HOUR => plural(n / WEEK, "week"),
            n if n > 6 * DAY + 12 * HOUR => "a week".into(),
            n if n > 36 * HOUR => plural(n / DAY, "day"),
            n if n > 22 * HOUR => "a day".into(),
            n if n > 90 * MINUTE => plural(n / HOUR, "hour"),
            n if n > 45 * MINUTE => "an hour".into(),
            n if n > 90 => plural(n / MINUTE, "minute"),
            n if n > 45 => "a minute".into(),
            n if n > 10 => format!("{n} seconds"),
            _ => return f.pad("now"),
        };
        if self.nanos > 0 {
            f.pad(&format!("in {text}"))
        } else {
            f.pad(&format!("{text} ago"))
        }
    }
}

/// `count` of `unit`, at least 2, since a count of 1 has its own wording above.
fn plural(count: i128, unit: &str) -> String {
    format!("{} {unit}s", count.max(2))
}
