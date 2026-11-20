//! Nushell's date value: an instant, and the UTC offset it is shown in.
//!
//! [`DateTime`] is built on [`jiff`]. Its text forms (`Display`, `Debug`, RFC 3339, RFC 2822,
//! serde and `strftime` output) match what nushell printed when dates were
//! `chrono::DateTime<FixedOffset>`, so scripts, saved files and plugins see the same text.

#[cfg(feature = "chrono")]
mod chrono_compat;
mod format;
mod humanize;
mod parse;

pub use format::DateFormatError;
pub use humanize::HumanTime;
pub use parse::{DateParseError, parse_utc_offset};

use jiff::{
    SignedDuration, Timestamp, ZonedDateTime, civil,
    tz::{Offset, TimeZone},
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    cmp::Ordering,
    fmt::{self, Write},
    hash::{Hash, Hasher},
    str::FromStr,
    time::SystemTime,
};

/// The `LC_TIME` locale named by the environment, read through `env`, or `en_US` when the name
/// isn't one pure-rust-locales knows.
pub fn time_locale<'a, F, O>(env: F) -> pure_rust_locales::Locale
where
    F: 'a + Fn(&str) -> Option<O>,
    O: Into<std::borrow::Cow<'a, str>>,
{
    nu_utils::get_locale_from_env_vars(Some("LC_TIME"), env)
        .and_then(|name| pure_rust_locales::Locale::try_from(name.as_ref()).ok())
        .unwrap_or(pure_rust_locales::Locale::en_US)
}

/// An instant in time together with the UTC offset it is displayed in.
///
/// Two dates are equal, ordered and hashed by their instant alone, so
/// `2001-02-03T04:05:06+05:30 == 2001-02-02T22:35:06Z`.
#[derive(Clone, Copy)]
pub struct DateTime {
    timestamp: Timestamp,
    offset: Offset,
}

/// How many fraction digits [`DateTime::to_rfc3339_opts`] writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecondsFormat {
    /// No fraction.
    Secs,
    /// Exactly 3 digits.
    Millis,
    /// Exactly 6 digits.
    Micros,
    /// Exactly 9 digits.
    Nanos,
    /// None when the fraction is zero, otherwise the shortest of 3, 6 or 9 digits.
    AutoSi,
}

impl DateTime {
    /// 1970-01-01T00:00:00Z.
    pub const UNIX_EPOCH: Self = Self::new(Timestamp::UNIX_EPOCH, Offset::UTC);

    /// The date at `timestamp`, shown at `offset`.
    pub const fn new(timestamp: Timestamp, offset: Offset) -> Self {
        Self { timestamp, offset }
    }

    /// The instant and offset of a jiff zoned date-time. The time zone itself is dropped.
    pub fn from_zoned(zoned: &ZonedDateTime) -> Self {
        Self::new(zoned.timestamp(), zoned.offset())
    }

    /// The wall-clock `datetime` at `offset`, or `None` when it falls outside jiff's range.
    pub fn from_civil(datetime: civil::DateTime, offset: Offset) -> Option<Self> {
        offset
            .to_timestamp(datetime)
            .ok()
            .map(|timestamp| Self::new(timestamp, offset))
    }

    /// The current time in the system time zone.
    pub fn now() -> Self {
        Self::from_zoned(&ZonedDateTime::now())
    }

    /// The current time at UTC.
    pub fn now_utc() -> Self {
        Self::new(Timestamp::now(), Offset::UTC)
    }

    /// `timestamp`, shown at the system time zone's offset for that instant.
    pub fn from_timestamp_local(timestamp: Timestamp) -> Self {
        Self::new(timestamp, TimeZone::system().to_offset(timestamp))
    }

    /// Nanoseconds since the Unix epoch, shown at `offset`.
    pub fn from_unix_nanos(nanos: i128, offset: Offset) -> Option<Self> {
        Timestamp::from_nanosecond(nanos)
            .ok()
            .map(|timestamp| Self::new(timestamp, offset))
    }

    /// Milliseconds since the Unix epoch, shown at `offset`.
    pub fn from_unix_millis(millis: i64, offset: Offset) -> Option<Self> {
        Timestamp::from_millisecond(millis)
            .ok()
            .map(|timestamp| Self::new(timestamp, offset))
    }

    /// Seconds and nanoseconds since the Unix epoch, shown at `offset`.
    pub fn from_unix_seconds(seconds: i64, nanos: i32, offset: Offset) -> Option<Self> {
        Timestamp::new(seconds, nanos)
            .ok()
            .map(|timestamp| Self::new(timestamp, offset))
    }

    /// A file or clock time, shown in the system time zone.
    pub fn from_system_time(time: SystemTime) -> Option<Self> {
        Timestamp::try_from(time)
            .ok()
            .map(Self::from_timestamp_local)
    }

    /// The instant.
    pub const fn timestamp(self) -> Timestamp {
        self.timestamp
    }

    /// The offset the date is shown at.
    pub const fn offset(self) -> Offset {
        self.offset
    }

    /// The wall-clock date and time at the offset.
    pub fn to_civil(self) -> civil::DateTime {
        self.offset.to_datetime(self.timestamp)
    }

    /// The year at the offset.
    pub fn year(self) -> i16 {
        self.to_civil().year()
    }

    /// A jiff zoned date-time in a fixed time zone at this offset.
    pub fn to_zoned(self) -> ZonedDateTime {
        self.timestamp
            .to_zoned_datetime(TimeZone::fixed(self.offset))
    }

    /// The same instant, shown at `offset`.
    pub const fn with_offset(self, offset: Offset) -> Self {
        Self::new(self.timestamp, offset)
    }

    /// The same instant, shown at `time_zone`'s offset for that instant.
    pub fn to_time_zone(self, time_zone: &TimeZone) -> Self {
        self.with_offset(time_zone.to_offset(self.timestamp))
    }

    /// The same instant, shown in the system time zone.
    pub fn to_local(self) -> Self {
        Self::from_timestamp_local(self.timestamp)
    }

    /// Nanoseconds since the Unix epoch.
    pub fn unix_nanos(self) -> i128 {
        self.timestamp.as_nanosecond()
    }

    /// The instant as a `SystemTime`, or `None` when the platform can't hold it.
    pub fn to_system_time(self) -> Option<SystemTime> {
        SystemTime::try_from(self.timestamp).ok()
    }

    /// The date `nanos` nanoseconds later, at the same offset, or `None` past jiff's range.
    pub fn checked_add_nanos(self, nanos: i64) -> Option<Self> {
        self.timestamp
            .checked_add(SignedDuration::from_nanos(nanos))
            .ok()
            .map(|timestamp| self.with_timestamp(timestamp))
    }

    /// The date `nanos` nanoseconds earlier, at the same offset, or `None` past jiff's range.
    pub fn checked_sub_nanos(self, nanos: i64) -> Option<Self> {
        self.timestamp
            .checked_sub(SignedDuration::from_nanos(nanos))
            .ok()
            .map(|timestamp| self.with_timestamp(timestamp))
    }

    /// Nanoseconds from `earlier` to `self`, negative when `self` comes first.
    pub fn nanos_since(self, earlier: Self) -> i128 {
        self.unix_nanos() - earlier.unix_nanos()
    }

    const fn with_timestamp(self, timestamp: Timestamp) -> Self {
        Self::new(timestamp, self.offset)
    }

    /// RFC 3339 with the shortest of 0, 3, 6 or 9 fraction digits and `+00:00` for UTC,
    /// for example `2001-02-03T04:05:06.100+05:30`.
    pub fn to_rfc3339(self) -> String {
        self.to_rfc3339_opts(SecondsFormat::AutoSi, false)
    }

    /// RFC 3339 with the fraction digits `seconds` asks for, and `Z` for UTC when `use_z`.
    pub fn to_rfc3339_opts(self, seconds: SecondsFormat, use_z: bool) -> String {
        let mut out = String::with_capacity(35);
        let civil = self.to_civil();
        write_civil_date(&mut out, civil, '-');
        out.push('T');
        write_civil_time(&mut out, civil);
        write_fraction(&mut out, civil.subsec_nanosecond(), seconds, ".");
        if use_z && self.offset.seconds() == 0 {
            out.push('Z');
        } else {
            write_offset(&mut out, self.offset, OffsetStyle::Minutes { colon: true });
        }
        out
    }

    /// RFC 2822, for example `Sat, 3 Feb 2001 04:05:06 +0530`. Years before 0 keep their sign,
    /// which RFC 2822 can't express; callers show those dates as RFC 3339 instead.
    pub fn to_rfc2822(self) -> String {
        let civil = self.to_civil();
        let mut out = String::with_capacity(31);
        out.push_str(format::SHORT_WEEKDAYS[weekday_from_sunday(civil)]);
        let _ = write!(out, ", {} ", civil.day());
        out.push_str(format::SHORT_MONTHS[civil.month() as usize - 1]);
        out.push(' ');
        write_year(&mut out, civil.year());
        out.push(' ');
        write_civil_time(&mut out, civil);
        out.push(' ');
        write_offset(&mut out, self.offset, OffsetStyle::Minutes { colon: false });
        out
    }

    /// Format with a `strftime` string, using `locale` for names, `%c %x %X %r` and the
    /// decimal point. See [`format`](self::format) for the specifiers.
    pub fn format(
        self,
        format: &str,
        locale: pure_rust_locales::Locale,
    ) -> Result<String, DateFormatError> {
        format::format(self, format, locale)
    }

    /// Parse strict RFC 3339 the way nushell's date literals always have: `T`, `t` or a space
    /// between date and time, any number of fraction digits, `Z`, `z` or `±HH:MM`, and `:60`.
    pub fn parse_rfc3339(text: &str) -> Result<Self, DateParseError> {
        parse::parse_rfc3339(text)
    }
}

/// The Unix epoch at UTC, as chrono's default was.
impl Default for DateTime {
    fn default() -> Self {
        Self::UNIX_EPOCH
    }
}

impl PartialEq for DateTime {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp
    }
}

impl Eq for DateTime {}

impl PartialOrd for DateTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DateTime {
    fn cmp(&self, other: &Self) -> Ordering {
        self.timestamp.cmp(&other.timestamp)
    }
}

impl Hash for DateTime {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.timestamp.hash(state);
    }
}

/// `2001-02-03 04:05:06.100 +05:30`; an offset with seconds keeps them (`+05:30:15`).
impl fmt::Display for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let civil = self.to_civil();
        let mut out = String::with_capacity(36);
        write_civil_date(&mut out, civil, '-');
        out.push(' ');
        write_civil_time(&mut out, civil);
        write_fraction(
            &mut out,
            civil.subsec_nanosecond(),
            SecondsFormat::AutoSi,
            ".",
        );
        out.push(' ');
        write_offset(&mut out, self.offset, OffsetStyle::Full);
        f.write_str(&out)
    }
}

/// `2001-02-03T04:05:06.100+05:30`; an offset with seconds keeps them (`+05:30:15`).
impl fmt::Debug for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let civil = self.to_civil();
        let mut out = String::with_capacity(35);
        write_civil_date(&mut out, civil, '-');
        out.push('T');
        write_civil_time(&mut out, civil);
        write_fraction(
            &mut out,
            civil.subsec_nanosecond(),
            SecondsFormat::AutoSi,
            ".",
        );
        write_offset(&mut out, self.offset, OffsetStyle::Full);
        f.write_str(&out)
    }
}

/// Reads relaxed RFC 3339: what [`DateTime::parse_rfc3339`] reads, plus unpadded fields, signed
/// years, spaces between parts, `UTC` and offsets without a colon.
impl FromStr for DateTime {
    type Err = DateParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        parse::parse_rfc3339_relaxed(text)
    }
}

/// Written as RFC 3339 with `Z` for UTC and the shortest of 0, 3, 6 or 9 fraction digits,
/// for example `"2001-02-03T04:05:06.100+05:30"`. Plugins have always received this text.
impl Serialize for DateTime {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_rfc3339_opts(SecondsFormat::AutoSi, true))
    }
}

/// Reads the relaxed RFC 3339 that [`FromStr`] reads, so plugins written in any language can
/// send their own RFC 3339 text.
impl<'de> Deserialize<'de> for DateTime {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;

        impl serde::de::Visitor<'_> for Visitor {
            type Value = DateTime;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("an RFC 3339 date-time string")
            }

            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<DateTime, E> {
                value.parse().map_err(E::custom)
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

/// How [`write_offset`] writes an offset.
#[derive(Clone, Copy)]
enum OffsetStyle {
    /// `+HH:MM`, or `+HH:MM:SS` when the offset has seconds.
    Full,
    /// Rounded to the minute, `+HH:MM` or `+HHMM`.
    Minutes { colon: bool },
    /// `+HH:MM:SS` or `+HHMMSS`, always with seconds.
    Seconds { colon: bool },
    /// Truncated to the hour, `+HH`.
    Hours,
}

fn write_offset(out: &mut String, offset: Offset, style: OffsetStyle) {
    let total = offset.seconds();
    out.push(if total < 0 { '-' } else { '+' });
    let total = total.unsigned_abs();
    let (hours, minutes, seconds, colon) = match style {
        OffsetStyle::Full => (total / 3600, total / 60 % 60, total % 60, true),
        OffsetStyle::Minutes { colon } => {
            let rounded = (total + 30) / 60;
            (rounded / 60, rounded % 60, 0, colon)
        }
        OffsetStyle::Seconds { colon } => (total / 3600, total / 60 % 60, total % 60, colon),
        OffsetStyle::Hours => (total / 3600, 0, 0, false),
    };
    let _ = write!(out, "{hours:02}");
    if matches!(style, OffsetStyle::Hours) {
        return;
    }
    if colon {
        out.push(':');
    }
    let _ = write!(out, "{minutes:02}");
    let with_seconds = match style {
        OffsetStyle::Full => seconds != 0,
        OffsetStyle::Seconds { .. } => true,
        _ => false,
    };
    if with_seconds {
        if colon {
            out.push(':');
        }
        let _ = write!(out, "{seconds:02}");
    }
}

/// A year as chrono wrote it: 4 digits, with `-` before negative years.
fn write_year(out: &mut String, year: i16) {
    if year < 0 {
        let _ = write!(out, "-{:04}", year.unsigned_abs());
    } else {
        let _ = write!(out, "{year:04}");
    }
}

fn write_civil_date(out: &mut String, civil: civil::DateTime, separator: char) {
    write_year(out, civil.year());
    let _ = write!(
        out,
        "{separator}{:02}{separator}{:02}",
        civil.month(),
        civil.day()
    );
}

fn write_civil_time(out: &mut String, civil: civil::DateTime) {
    let _ = write!(
        out,
        "{:02}:{:02}:{:02}",
        civil.hour(),
        civil.minute(),
        civil.second()
    );
}

/// Writes `nanos` as a fraction after `point`, with the digits `seconds` asks for.
fn write_fraction(out: &mut String, nanos: i32, seconds: SecondsFormat, point: &str) {
    let digits = match seconds {
        SecondsFormat::Secs => return,
        SecondsFormat::Millis => 3,
        SecondsFormat::Micros => 6,
        SecondsFormat::Nanos => 9,
        SecondsFormat::AutoSi if nanos == 0 => return,
        SecondsFormat::AutoSi if nanos % 1_000_000 == 0 => 3,
        SecondsFormat::AutoSi if nanos % 1_000 == 0 => 6,
        SecondsFormat::AutoSi => 9,
    };
    out.push_str(point);
    let value = nanos / 10_i32.pow(9 - digits);
    let _ = write!(out, "{value:0width$}", width = digits as usize);
}

/// 0 for Sunday through 6 for Saturday.
fn weekday_from_sunday(civil: civil::DateTime) -> usize {
    civil.weekday().to_sunday_zero_offset() as usize
}

#[cfg(test)]
mod tests;
