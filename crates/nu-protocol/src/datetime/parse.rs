// The UTC offset readers in this file follow those of chrono 0.4.45
// (https://github.com/chronotope/chrono). chrono is dual-licensed under the MIT License and the
// Apache License 2.0; this file uses it under the MIT License:
//
// Copyright (c) 2014--2026, Kang Seonghoon and contributors.
//
// The MIT License (MIT)
//
// Copyright (c) 2014, Kang Seonghoon.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

//! Reading dates and UTC offsets from text.
//!
//! [`parse_rfc3339`] reads every date nushell takes as text without a format: date literals,
//! `FromStr` and serde. It is jiff's RFC 3339 and ISO 8601 reader, so `2001-02-03T04:05Z`
//! (no seconds), `+0530` and `-08` offsets and a `,` before the fraction read too. The text
//! needs an offset or `Z`. As jiff reads them, second 60 is second 59 and a fraction can have
//! at most 9 digits.
//!
//! The offset readers follow chrono's, as `into datetime` (a record's `timezone`) and
//! `date to-timezone` read offsets with chrono.

use super::DateTime;
use jiff::{civil, fmt::temporal::Pieces, tz::Offset};
use std::fmt;

/// Text that isn't a date-time or offset in the form being read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DateParseError {
    /// The text ends before the date-time or offset does.
    TooShort,
    /// A character doesn't fit the form.
    Invalid,
    /// A field is out of range, such as minute 61 or a date past year 9999.
    OutOfRange,
    /// The fields are not enough for a date and time, such as a year alone.
    NotEnough,
    /// The fields contradict each other, such as the offset `-0800` with the zone `UTC`.
    Impossible,
    /// jiff's reason the text isn't a date, such as `parsed date is not valid`.
    Jiff(String),
}

impl fmt::Display for DateParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::TooShort => "premature end of input",
            Self::Invalid => "input contains invalid characters",
            Self::OutOfRange => "input is out of range",
            Self::NotEnough => "input is not enough for unique date and time",
            Self::Impossible => "no possible date and time matching input",
            Self::Jiff(reason) => reason,
        })
    }
}

impl std::error::Error for DateParseError {}

impl From<jiff::Error> for DateParseError {
    fn from(error: jiff::Error) -> Self {
        Self::Jiff(error.to_string())
    }
}

type ParseResult<T> = Result<T, DateParseError>;

/// Reads RFC 3339 and the ISO 8601 forms jiff reads, with an offset or `Z`, ignoring spaces
/// around it. A date alone is midnight. A signed year may have 4 or 5 digits, as chrono wrote
/// them (`-0044-03-15T12:00:00+00:00`); jiff reads 6. A time zone in brackets
/// (`...Z[America/Chicago]`) is an error, since this reader would drop it; `into datetime`
/// reads such text with the zone.
pub(super) fn parse_rfc3339(text: &str) -> ParseResult<DateTime> {
    let text = text.trim();
    let digits = text.get(1..).map_or(0, |rest| {
        rest.bytes().take_while(u8::is_ascii_digit).count()
    });
    let widened;
    let text = match text.as_bytes() {
        [sign @ (b'+' | b'-'), ..] if (4..6).contains(&digits) => {
            widened = format!(
                "{}{:0>6}{}",
                char::from(*sign),
                &text[1..=digits],
                &text[1 + digits..]
            );
            widened.as_str()
        }
        _ => text,
    };
    let pieces = Pieces::parse(text)?;
    if pieces.time_zone_annotation().is_some() {
        return Err(DateParseError::Invalid);
    }
    let offset = pieces.to_numeric_offset().ok_or(DateParseError::TooShort)?;
    let offset = at_most_a_day(offset)?;
    let time = pieces.time().unwrap_or(civil::Time::midnight());
    DateTime::from_civil(pieces.date().to_datetime(time), offset).ok_or(DateParseError::OutOfRange)
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

/// Reads an offset: a sign (`+`, `-` or `−`), two hour digits, a separator `separator`
/// consumes, and two minute digits. `zulu` also reads `Z`/`z`, and `missing_minutes` the hours
/// alone when the separator ends the text. Returns the text left over and the offset in seconds
/// east of UTC.
pub(super) fn read_offset(
    mut s: &str,
    zulu: bool,
    missing_minutes: bool,
    separator: impl Fn(&str) -> ParseResult<&str>,
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
    s = separator(&s[2..])?;
    let (rest, minutes) = match (s.as_bytes().first(), s.as_bytes().get(1)) {
        (Some(b'0'..=b'5'), Some(b'0'..=b'9')) => (&s[2..], two_digits(s.as_bytes(), 0)?),
        (Some(b'6'..=b'9'), Some(b'0'..=b'9')) => return Err(DateParseError::OutOfRange),
        (Some(_), Some(_)) => return Err(DateParseError::Invalid),
        (None, _) if missing_minutes => (s, 0),
        _ => return Err(DateParseError::TooShort),
    };
    let seconds = (hours * 3600 + minutes * 60) as i32;
    Ok((rest, if negative { -seconds } else { seconds }))
}

/// Skips any run of colons and whitespace.
pub(super) fn colon_or_space(s: &str) -> ParseResult<&str> {
    Ok(s.trim_start_matches(|c: char| c == ':' || c.is_whitespace()))
}

/// The offset `seconds` east of UTC, which must be shorter than a day, as chrono's offsets were.
fn within_a_day(seconds: i32) -> ParseResult<Offset> {
    if seconds.unsigned_abs() >= 86_400 {
        return Err(DateParseError::OutOfRange);
    }
    Offset::from_seconds(seconds).map_err(|_| DateParseError::OutOfRange)
}

/// `offset` if it is at most a day from UTC. jiff reads offsets up to ±25:59:59, which the other
/// readers (records, YAML, TOML, `date to-timezone`) refuse. A whole day stays, since `to nuon`
/// writes `+23:59:59` rounded to the minute as `+24:00`.
pub(super) fn at_most_a_day(offset: Offset) -> ParseResult<Offset> {
    if offset.seconds().unsigned_abs() > 86_400 {
        return Err(DateParseError::OutOfRange);
    }
    Ok(offset)
}

/// Reads the `:SS` that may follow an offset's minutes, as `into record` and
/// `format date '%::z'` write it, with the sign of the offset `text` starts with. Returns the
/// text left over and the seconds.
fn offset_seconds<'a>(text: &str, rest: &'a str) -> (&'a str, i32) {
    match rest.strip_prefix(':').map(str::as_bytes) {
        Some([a @ b'0'..=b'5', b @ b'0'..=b'9', ..]) => {
            let seconds = i32::from((a - b'0') * 10 + (b - b'0'));
            let negative = text.starts_with(['-', '\u{2212}']);
            (&rest[3..], if negative { -seconds } else { seconds })
        }
        _ => (rest, 0),
    }
}

/// Reads a UTC offset as chrono's `FixedOffset::from_str` did: `±HH:MM`, `±HHMM` or `±HH MM`,
/// shorter than a day, and also the `:SS` seconds `into record` writes (`+00:19:32`). Text
/// after the offset is ignored.
pub fn parse_utc_offset(text: &str) -> ParseResult<Offset> {
    let (rest, hours_and_minutes) = read_offset(text, false, false, colon_or_space)?;
    let (_, seconds) = offset_seconds(text, rest);
    within_a_day(hours_and_minutes + seconds)
}

/// Reads a time zone offset as `date to-timezone` does: `+HH`, `+HHMM`, `+HH:MM` or `+HH:MM:SS`
/// with an ASCII `+` or `-`, shorter than a day. Spaces may surround the one colon after the
/// hours and follow the offset. `Invalid` means the text doesn't start like an offset, so it
/// may be a time zone name.
pub fn parse_zone_offset(text: &str) -> ParseResult<Offset> {
    if text.starts_with('\u{2212}') {
        return Err(DateParseError::Invalid);
    }
    /// At most one colon, with spaces around it.
    fn one_colon(s: &str) -> ParseResult<&str> {
        let s = s.trim_start();
        match s.strip_prefix(':').map(str::trim_start) {
            Some("") => Err(DateParseError::Invalid),
            Some(rest) => Ok(rest),
            None => Ok(s),
        }
    }
    let (rest, hours_and_minutes) = read_offset(text, false, true, one_colon)?;
    let (rest, seconds) = offset_seconds(text, rest);
    if !rest.trim().is_empty() {
        return Err(DateParseError::Invalid);
    }
    within_a_day(hours_and_minutes + seconds)
}
