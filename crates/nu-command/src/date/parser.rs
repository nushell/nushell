// Modified from chrono::format::scan

use chrono::{DateTime, FixedOffset, Local, Offset, TimeZone};
use chrono_tz::Tz;
use titlecase::titlecase;

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum ParseErrorKind {
    /// Given field is out of permitted range.
    OutOfRange,

    /// The input string has some invalid character sequence for given formatting items.
    Invalid,

    /// The input string has been prematurely ended.
    TooShort,
}

pub fn datetime_in_timezone(
    dt: &DateTime<FixedOffset>,
    s: &str,
) -> Result<DateTime<FixedOffset>, ParseErrorKind> {
    match timezone_offset_internal(s, true, true) {
        Ok(offset) => match FixedOffset::east_opt(offset) {
            Some(offset) => Ok(dt.with_timezone(&offset)),
            None => Err(ParseErrorKind::OutOfRange),
        },
        Err(ParseErrorKind::Invalid) => {
            if s.to_lowercase() == "local" {
                Ok(dt.with_timezone(Local::now().offset()))
            } else {
                let tz: Tz = parse_timezone_internal(s)?;
                let offset = tz.offset_from_utc_datetime(&dt.naive_utc()).fix();
                Ok(dt.with_timezone(&offset))
            }
        }
        Err(e) => Err(e),
    }
}

fn parse_timezone_internal(s: &str) -> Result<Tz, ParseErrorKind> {
    if let Ok(tz) = s.parse() {
        Ok(tz)
    } else if let Ok(tz) = titlecase(s).parse() {
        Ok(tz)
    } else if let Ok(tz) = s.to_uppercase().parse() {
        Ok(tz)
    } else {
        Err(ParseErrorKind::Invalid)
    }
}

fn timezone_offset_internal(
    mut s: &str,
    consume_colon: bool,
    allow_missing_minutes: bool,
) -> Result<i32, ParseErrorKind> {
    fn digits(s: &str) -> Result<(u8, u8), ParseErrorKind> {
        let b = s.as_bytes();
        if b.len() < 2 {
            Err(ParseErrorKind::TooShort)
        } else {
            Ok((b[0], b[1]))
        }
    }
    let negative = match s.as_bytes().first() {
        Some(&b'+') => false,
        Some(&b'-') => true,
        Some(_) => return Err(ParseErrorKind::Invalid),
        None => return Err(ParseErrorKind::TooShort),
    };
    s = &s[1..];

    // hours (00--99)
    let hours = match digits(s)? {
        (h1 @ b'0'..=b'9', h2 @ b'0'..=b'9') => i32::from((h1 - b'0') * 10 + (h2 - b'0')),
        _ => return Err(ParseErrorKind::Invalid),
    };
    s = &s[2..];

    // colons (and possibly other separators)
    if consume_colon {
        s = s.trim_start_matches(|c: char| c == ':' || c.is_whitespace());
    }

    // minutes (00--59)
    // if the next two items are digits then we have to add minutes
    let minutes = if let Ok(ds) = digits(s) {
        match ds {
            (m1 @ b'0'..=b'5', m2 @ b'0'..=b'9') => i32::from((m1 - b'0') * 10 + (m2 - b'0')),
            (b'6'..=b'9', b'0'..=b'9') => return Err(ParseErrorKind::OutOfRange),
            _ => return Err(ParseErrorKind::Invalid),
        }
    } else if allow_missing_minutes {
        0
    } else {
        return Err(ParseErrorKind::TooShort);
    };
    match s.len() {
        len if len >= 2 => &s[2..],
        len if len == 0 => s,
        _ => return Err(ParseErrorKind::TooShort),
    };

    let seconds = hours * 3600 + minutes * 60;
    Ok(if negative { -seconds } else { seconds })
}
