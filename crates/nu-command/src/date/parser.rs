//! Reading a time zone given as an offset (`+05:30`), `local`, or an IANA name.

use jiff::tz::Offset;
use nu_protocol::DateTime;
use std::sync::LazyLock;

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum ParseErrorKind {
    /// Given field is out of permitted range.
    OutOfRange,

    /// The input string has some invalid character sequence for given formatting items.
    Invalid,

    /// The input string has been prematurely ended.
    TooShort,
}

/// The IANA time zone names jiff's database knows, sorted, for completions and
/// `date list-timezone`.
pub(crate) static TIME_ZONE_NAMES: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    let mut names: Vec<&'static str> = jiff::tz::db()
        .available()
        .map(|name| &*name.as_str().to_owned().leak())
        .collect();
    // `posixrules` is a rules file and `Factory` a placeholder; neither was listed before.
    names.retain(|name| !matches!(*name, "posixrules" | "Factory"));
    names.sort_unstable();
    names.dedup();
    names
});

/// `date` shown in the time zone `s` names: an offset such as `+05:30`, `local`, or an IANA
/// name in any case.
///
/// `local` uses the system time zone's offset now, not at `date`, as nushell always has.
pub fn datetime_in_timezone(date: DateTime, s: &str) -> Result<DateTime, ParseErrorKind> {
    match timezone_offset_internal(s, true, true) {
        // chrono's offsets, which nushell used, stop short of a day.
        Ok(offset) if offset.unsigned_abs() >= 86_400 => Err(ParseErrorKind::OutOfRange),
        Ok(offset) => Offset::from_seconds(offset)
            .map(|offset| date.with_offset(offset))
            .map_err(|_| ParseErrorKind::OutOfRange),
        Err(ParseErrorKind::Invalid) => {
            if s.eq_ignore_ascii_case("local") {
                Ok(date.with_offset(DateTime::now().offset()))
            } else {
                // The name is trimmed for the newline that ends `/etc/timezone`.
                let time_zone = jiff::tz::db()
                    .get(s.trim())
                    .map_err(|_| ParseErrorKind::Invalid)?;
                Ok(date.to_time_zone(&time_zone))
            }
        }
        Err(e) => Err(e),
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

    // Optional colon, with whitespace permitted around it.
    if consume_colon {
        s = s.trim_start();
        if let Some(rest) = s.strip_prefix(':') {
            s = rest.trim_start();
            if s.is_empty() {
                return Err(ParseErrorKind::Invalid);
            }
        }
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
    let suffix = match s.len() {
        len if len >= 2 => &s[2..],
        0 => s,
        _ => return Err(ParseErrorKind::TooShort),
    };
    // Preserve seconds (00--59) emitted by `into record` and `format date "%::z"`.
    let (offset_seconds, suffix) = match suffix.strip_prefix(':').map(digits) {
        Some(Ok((s1 @ b'0'..=b'5', s2 @ b'0'..=b'9'))) => {
            (i32::from((s1 - b'0') * 10 + (s2 - b'0')), &suffix[3..])
        }
        _ => (0, suffix),
    };
    if !suffix.trim().is_empty() {
        return Err(ParseErrorKind::Invalid);
    }

    let seconds = hours * 3600 + minutes * 60 + offset_seconds;
    Ok(if negative { -seconds } else { seconds })
}
