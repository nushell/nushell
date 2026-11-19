//! RFC 3339 readers for [`DateTime`] that accept exactly what nushell accepted with chrono:
//! the strict form for date literals and the relaxed form for `FromStr` and serde.
//!
//! A leap second (`23:59:60`) reads as `23:59:59` with the same fraction, since jiff has no
//! leap seconds.

use super::DateTime;
use jiff::{civil, tz::Offset};
use std::fmt;

/// Text that isn't a date-time in the form being read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateParseError {
    /// The text ends before the date-time does.
    TooShort,
    /// The text continues after the date-time.
    TooLong,
    /// A character doesn't fit the form.
    Invalid,
    /// A field is out of range, such as month 13 or year 10000.
    OutOfRange,
}

impl fmt::Display for DateParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::TooShort => "premature end of input",
            Self::TooLong => "trailing input",
            Self::Invalid => "input contains invalid characters",
            Self::OutOfRange => "input is out of range",
        })
    }
}

impl std::error::Error for DateParseError {}

type ParseResult<T> = Result<T, DateParseError>;

/// The fields read from the text, before they are checked as a date.
struct Fields {
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    minute: i64,
    second: i64,
    nanosecond: i64,
    offset_seconds: i32,
}

impl Fields {
    fn into_date_time(self) -> ParseResult<DateTime> {
        let small = |v: i64, max: i64| {
            if (0..=max).contains(&v) {
                Ok(v as i8)
            } else {
                Err(DateParseError::OutOfRange)
            }
        };
        let year = i16::try_from(self.year).map_err(|_| DateParseError::OutOfRange)?;
        // A leap second keeps its fraction on the last ordinary second.
        let second = if self.second == 60 { 59 } else { self.second };
        let datetime = civil::DateTime::new(
            year,
            small(self.month, 12)?,
            small(self.day, 31)?,
            small(self.hour, 23)?,
            small(self.minute, 59)?,
            small(second, 59)?,
            i32::try_from(self.nanosecond).map_err(|_| DateParseError::OutOfRange)?,
        )
        .map_err(|_| DateParseError::OutOfRange)?;
        // chrono's offsets stop short of a full day.
        if self.offset_seconds.unsigned_abs() >= 86_400 {
            return Err(DateParseError::OutOfRange);
        }
        let offset =
            Offset::from_seconds(self.offset_seconds).map_err(|_| DateParseError::OutOfRange)?;
        DateTime::from_civil(datetime, offset).ok_or(DateParseError::OutOfRange)
    }
}

/// Reads 1 to `max` ASCII digits.
fn number(s: &str, max: usize) -> ParseResult<(&str, i64)> {
    let len = s.bytes().take(max).take_while(u8::is_ascii_digit).count();
    if len == 0 {
        return Err(if s.is_empty() {
            DateParseError::TooShort
        } else {
            DateParseError::Invalid
        });
    }
    let value = s[..len].parse().map_err(|_| DateParseError::OutOfRange)?;
    Ok((&s[len..], value))
}

/// Reads exactly two ASCII digits from the start of `bytes` at `at`.
fn two_digits(bytes: &[u8], at: usize) -> ParseResult<i64> {
    match (bytes.get(at), bytes.get(at + 1)) {
        (Some(a @ b'0'..=b'9'), Some(b @ b'0'..=b'9')) => {
            Ok(i64::from((a - b'0') * 10 + (b - b'0')))
        }
        (None, _) | (_, None) => Err(DateParseError::TooShort),
        _ => Err(DateParseError::Invalid),
    }
}

/// Reads fraction digits after a `.`: the first 9 count, the rest are skipped.
fn fraction(s: &str) -> ParseResult<(&str, i64)> {
    let (rest, value) = number(s, 9)?;
    let digits = s.len() - rest.len();
    let scaled = value * 10_i64.pow(9 - digits as u32);
    Ok((
        rest.trim_start_matches(|c: char| c.is_ascii_digit()),
        scaled,
    ))
}

/// Reads a signed `HH` + separator + `MM` offset, or `Z`/`z` when `zulu`. `colon` consumes the
/// separator.
fn offset(
    mut s: &str,
    zulu: bool,
    colon: impl Fn(&str) -> ParseResult<&str>,
) -> ParseResult<(&str, i32)> {
    if zulu && let Some(rest) = s.strip_prefix(['Z', 'z']) {
        return Ok((rest, 0));
    }
    let negative = match s.chars().next() {
        Some('+') => false,
        Some('-' | '\u{2212}') => true,
        Some(_) => return Err(DateParseError::Invalid),
        None => return Err(DateParseError::TooShort),
    };
    s = &s[s.chars().next().map_or(0, char::len_utf8)..];
    let hours = two_digits(s.as_bytes(), 0)?;
    s = colon(&s[2..])?;
    let minutes = match (s.as_bytes().first(), s.as_bytes().get(1)) {
        (Some(b'0'..=b'5'), Some(b'0'..=b'9')) => two_digits(s.as_bytes(), 0)?,
        (Some(b'6'..=b'9'), Some(b'0'..=b'9')) => return Err(DateParseError::OutOfRange),
        (Some(_), Some(_)) => return Err(DateParseError::Invalid),
        _ => return Err(DateParseError::TooShort),
    };
    let seconds = (hours * 3600 + minutes * 60) as i32;
    Ok((&s[2..], if negative { -seconds } else { seconds }))
}

/// Skips any run of colons and whitespace.
fn colon_or_space(s: &str) -> ParseResult<&str> {
    Ok(s.trim_start_matches(|c: char| c == ':' || c.is_whitespace()))
}

/// Reads a UTC offset as chrono's `FixedOffset::from_str` did: `±HH:MM`, `±HHMM` or `±HH MM`,
/// shorter than a day. Text after the offset is ignored.
pub fn parse_utc_offset(text: &str) -> ParseResult<Offset> {
    let (_, seconds) = offset(text, false, colon_or_space)?;
    if seconds.unsigned_abs() >= 86_400 {
        return Err(DateParseError::OutOfRange);
    }
    Offset::from_seconds(seconds).map_err(|_| DateParseError::OutOfRange)
}

/// Strict RFC 3339, as `chrono::DateTime::parse_from_rfc3339` read it.
pub(super) fn parse_rfc3339(s: &str) -> ParseResult<DateTime> {
    let bytes = s.as_bytes();
    if bytes.len() < 19 {
        return Err(DateParseError::TooShort);
    }
    let separator = |at: usize, expected: &[u8]| {
        if expected.contains(&bytes[at]) {
            Ok(())
        } else {
            Err(DateParseError::Invalid)
        }
    };
    let year = two_digits(bytes, 0)? * 100 + two_digits(bytes, 2)?;
    separator(4, b"-")?;
    let month = two_digits(bytes, 5)?;
    separator(7, b"-")?;
    let day = two_digits(bytes, 8)?;
    // The date is checked before the rest is read.
    if civil::Date::new(year as i16, month as i8, day as i8).is_err() || month == 0 || day == 0 {
        return Err(DateParseError::OutOfRange);
    }
    separator(10, b"tT ")?;
    let hour = two_digits(bytes, 11)?;
    separator(13, b":")?;
    let minute = two_digits(bytes, 14)?;
    separator(16, b":")?;
    let second = two_digits(bytes, 17)?;
    let mut rest = &s[19..];
    let mut nanosecond = 0;
    if let Some(after_dot) = rest.strip_prefix('.') {
        (rest, nanosecond) = fraction(after_dot)?;
    }
    let (rest, offset_seconds) = offset(rest, true, |s| {
        s.strip_prefix(':').ok_or(if s.is_empty() {
            DateParseError::TooShort
        } else {
            DateParseError::Invalid
        })
    })?;
    if !rest.is_empty() {
        return Err(DateParseError::TooLong);
    }
    Fields {
        year,
        month,
        day,
        hour,
        minute,
        second,
        nanosecond,
        offset_seconds,
    }
    .into_date_time()
}

/// Relaxed RFC 3339, as chrono's `FromStr for DateTime<FixedOffset>` read it: fields need no
/// padding, a year with a sign can have any number of digits, whitespace may sit between
/// parts, the offset's colon is optional, and `UTC` is an offset.
pub(super) fn parse_rfc3339_relaxed(s: &str) -> ParseResult<DateTime> {
    fn literal(s: &str, expected: char) -> ParseResult<&str> {
        let s = s.trim_start();
        s.strip_prefix(expected).ok_or(if s.is_empty() {
            DateParseError::TooShort
        } else {
            DateParseError::Invalid
        })
    }
    fn field(s: &str, width: usize) -> ParseResult<(&str, i64)> {
        number(s.trim_start(), width)
    }

    let s = s.trim_start();
    let (s, year) = if let Some(digits) = s.strip_prefix('-') {
        let (s, v) = number(digits, usize::MAX)?;
        (s, -v)
    } else if let Some(digits) = s.strip_prefix('+') {
        number(digits, usize::MAX)?
    } else {
        number(s, 4)?
    };
    let s = literal(s, '-')?;
    let (s, month) = field(s, 2)?;
    let s = literal(s, '-')?;
    let (s, day) = field(s, 2)?;
    let s = match s.as_bytes().first() {
        Some(b't' | b'T' | b' ') => &s[1..],
        Some(_) => return Err(DateParseError::Invalid),
        None => return Err(DateParseError::TooShort),
    };
    let (s, hour) = field(s, 2)?;
    let s = literal(s, ':')?;
    let (s, minute) = field(s, 2)?;
    let s = literal(s, ':')?;
    let (mut s, second) = field(s, 2)?;
    let mut nanosecond = 0;
    if let Some(after_dot) = s.strip_prefix('.') {
        (s, nanosecond) = fraction(after_dot)?;
    }
    let s = s.trim_start();
    let (s, offset_seconds) = if s.len() >= 3 && s.as_bytes()[..3].eq_ignore_ascii_case(b"UTC") {
        (&s[3..], 0)
    } else {
        offset(s, true, colon_or_space)?
    };
    if !s.trim_start().is_empty() {
        return Err(DateParseError::TooLong);
    }
    Fields {
        year,
        month,
        day,
        hour,
        minute,
        second,
        nanosecond,
        offset_seconds,
    }
    .into_date_time()
}
