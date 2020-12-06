// Modified from chrono::format::scan

use chrono::{FixedOffset, Local};

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum ParseErrorKind {
    /// Given field is out of permitted range.
    OutOfRange,

    /// The input string has some invalid character sequence for given formatting items.
    Invalid,

    /// The input string has been prematurely ended.
    TooShort,

    /// The timezone abbreviation is either invalid or not yet supported.
    NotSupported,
}

pub fn get_timezone_offset(s: &str) -> Result<FixedOffset, ParseErrorKind> {
    let offset_hours = |o| Ok(FixedOffset::east(o * 3600));

    if s.chars().all(|x| x.is_ascii_alphabetic()) {
        match s.to_lowercase().as_str() {
            "gmt" | "utc" | "ut" => offset_hours(0),
            "edt" => offset_hours(-4),
            "est" | "cdt" => offset_hours(-5),
            "cst" | "mdt" => offset_hours(-6),
            "mst" | "pdt" => offset_hours(-7),
            "pst" => offset_hours(-8),
            "local" => Ok(*Local::now().offset()),
            _ => Err(ParseErrorKind::NotSupported),
        }
    } else {
        let offset = timezone_offset_internal(s, true, true)?;

        match FixedOffset::east_opt(offset) {
            Some(offset) => Ok(offset),
            None => Err(ParseErrorKind::OutOfRange),
        }
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
