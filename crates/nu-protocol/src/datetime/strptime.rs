//! `strftime`-style reading for `into datetime --format`, `seq date --input-format` and
//! `from kdl`, on jiff's parser.
//!
//! jiff reads most specifiers the way chrono did. The ones it doesn't read take chrono's
//! meaning in ones it does before jiff sees the format: `%c` is `%a %b %e %H:%M:%S %Y`, `%x` is
//! `%m/%d/%y`, `%X` is `%H:%M:%S`, `%r` is `%I:%M:%S %p`, `%v` is `%e-%b-%Y`, `%+` is RFC 3339,
//! and nushell's own `%J` and `%Q` are `%Y%m%d` and `%H%M%S`. As with chrono, `%z`, `%:z`,
//! `%::z` and `%#z` read an offset with or without colons (`+0530`, `+05:30`, `+05`), `%Z`
//! reads an offset, `UTC` or `GMT`, and a `Z` that ends the text reads as the offset `+00:00`.
//!
//! The rest is jiff's reading: second 60 is second 59, a field read twice keeps the last value,
//! `%s` wins over a date and time, two-digit years are 1969 to 2068, and `%f` reads fraction
//! digits, so `123` is 0.123 seconds.

use super::{DateParseError, DateTime};
use jiff::{Timestamp, civil, fmt::strtime::BrokenDownTime, tz::Offset};

/// The fields a text has in a `strftime`-style format.
#[derive(Debug)]
pub struct ParsedDate(BrokenDownTime);

impl ParsedDate {
    /// Reads all of `text` with `format`.
    pub fn parse(text: &str, format: &str) -> Result<Self, DateParseError> {
        if format == "%+" {
            let date = DateTime::parse_rfc3339(text)?;
            let mut fields = BrokenDownTime::from(date.to_civil());
            fields.set_offset(Some(date.offset()));
            return Ok(Self(fields));
        }
        // jiff's `%z` reads `+0530` and its `%:::z` reads `+05:30` and `+05`; chrono read all
        // three for any of its offset specifiers.
        let has = |offset: &str, zone: &str| to_jiff_format(format, offset, zone).contains('\0');
        let offsets: &[&str] = if has("\0", "") {
            &["%z", "%:::z"]
        } else {
            &[""]
        };
        let zones: &[&str] = if has("", "\0") {
            &["%z", "%:::z", "UTC", "GMT", "utc", "gmt"]
        } else {
            &[""]
        };
        let zulu = text
            .strip_suffix(['Z', 'z'])
            .map(|rest| format!("{rest}+00:00"));
        let mut error = None;
        for offset in offsets {
            for zone in zones {
                let jiff_format = to_jiff_format(format, offset, zone);
                for text in std::iter::once(text).chain(zulu.as_deref()) {
                    match BrokenDownTime::parse(&jiff_format, text) {
                        Ok(mut fields) => {
                            if !zone.starts_with('%') && !zone.is_empty() {
                                fields.set_offset(Some(Offset::UTC));
                            }
                            return Ok(Self(fields));
                        }
                        Err(err) => {
                            error.get_or_insert(err);
                        }
                    }
                }
            }
        }
        // jiff's reason for the first reading of `%Z`, as an offset, would mislead.
        match error {
            Some(error) if zones.len() == 1 => Err(error.into()),
            _ => Err(DateParseError::Invalid),
        }
    }

    /// The offset the text has, if any.
    pub fn offset(&self) -> Option<Offset> {
        self.0.offset()
    }

    /// The instant, when the text has a Unix timestamp, or an offset and a date. Otherwise
    /// [`DateParseError::NotEnough`].
    pub fn to_datetime(&self) -> Result<DateTime, DateParseError> {
        let fields = &self.0;
        let offset = fields.offset();
        if let Some(timestamp) = fields.timestamp() {
            // jiff keeps a fraction read with `%s` apart from the timestamp.
            let nanos = fields.subsec_nanosecond().unwrap_or(0);
            let seconds = timestamp.as_second();
            let nanos = if seconds < 0 { -nanos } else { nanos };
            let timestamp = Timestamp::new(seconds, nanos)?;
            return Ok(DateTime::new(timestamp, offset.unwrap_or(Offset::UTC)));
        }
        match offset {
            Some(offset) if self.has_date() => {
                let timestamp = offset.to_timestamp(fields.to_datetime()?)?;
                Ok(DateTime::new(timestamp, offset))
            }
            _ => Err(DateParseError::NotEnough),
        }
    }

    /// The wall-clock date and time, held at UTC; a date alone is midnight. Without a date,
    /// [`DateParseError::NotEnough`].
    pub fn to_wall_clock(&self) -> Result<DateTime, DateParseError> {
        if !self.has_date() {
            return Err(DateParseError::NotEnough);
        }
        DateTime::from_civil(self.0.to_datetime()?, Offset::UTC).ok_or(DateParseError::OutOfRange)
    }

    /// The time of day; midnight when the text has none.
    pub fn to_time(&self) -> Result<civil::Time, DateParseError> {
        Ok(self.0.to_time()?)
    }

    /// Whether the text names a date, or part of one: a time of day alone, perhaps with a
    /// weekday, is no date.
    fn has_date(&self) -> bool {
        let fields = &self.0;
        fields.year().is_some()
            || fields.month().is_some()
            || fields.day().is_some()
            || fields.day_of_year().is_some()
            || fields.iso_week_year().is_some()
            || fields.iso_week().is_some()
    }
}

/// `format` with the specifiers jiff doesn't read spelled in ones it does, its offset
/// specifiers (`%z`, `%:z`, `%::z`, `%:::z`, `%#z`) as `offset` and `%Z` as `zone`.
fn to_jiff_format(format: &str, offset: &str, zone: &str) -> String {
    let mut out = String::with_capacity(format.len() + 16);
    let mut chars = format.chars();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        let Some(spec) = chars.next() else {
            out.push('%');
            break;
        };
        match spec {
            'c' => out.push_str("%a %b %e %H:%M:%S %Y"),
            'x' => out.push_str("%m/%d/%y"),
            'X' => out.push_str("%H:%M:%S"),
            'r' => out.push_str("%I:%M:%S %p"),
            'v' => out.push_str("%e-%b-%Y"),
            '+' => out.push_str("%Y-%m-%dT%H:%M:%S%.f%:z"),
            'J' => out.push_str("%Y%m%d"),
            'Q' => out.push_str("%H%M%S"),
            'Z' => out.push_str(zone),
            'z' => out.push_str(offset),
            ':' | '#' => {
                let mut rest = chars.clone();
                while spec == ':' && rest.clone().next() == Some(':') {
                    rest.next();
                }
                if rest.next() == Some('z') {
                    chars = rest;
                    out.push_str(offset);
                } else {
                    out.push('%');
                    out.push(spec);
                }
            }
            other => {
                out.push('%');
                out.push(other);
            }
        }
    }
    out
}
