//! Reading text with a user's `strptime` format, for `into datetime --format` and `seq date`.
//!
//! jiff's `strptime` reads the fields. A pre-pass keeps the meanings these commands have always
//! had: `%J` and `%Q` are `%Y%m%d` and `%H%M%S`; `%c`, `%x`, `%X`, `%r`, `%+` and `%v` read the
//! POSIX layouts; `%Z` skips a time zone name (it doesn't set one); `%z` reads `+08:00`, `+0800`
//! or `+08 00`, and `%#z` also `Z` and `+08`; `%E`/`%O` modifiers are dropped; and a two-digit
//! year of `69` is 2069, as with any `%y` below 70.

use jiff::{civil, fmt::strtime::BrokenDownTime, tz::Offset};
use nu_protocol::DateTime;

/// What a format read from the text.
#[derive(Debug)]
pub(crate) enum Parsed {
    /// An instant: the text had an offset (`%z`) or a Unix timestamp (`%s`). A timestamp
    /// without an offset is at UTC.
    Instant(DateTime),
    /// A date and time with no offset.
    Civil(civil::DateTime),
    /// A date with no time.
    Date(civil::Date),
    /// A time with no date.
    Time(civil::Time),
}

/// Reads `text` with `format`, or returns why it can't.
pub(crate) fn parse_with_format(text: &str, format: &str) -> Result<Parsed, String> {
    let rewritten = rewrite_format(format);
    let mut tm = parse_segments(text, &rewritten)?;
    if rewritten.two_digit_year && tm.year() == Some(1969) {
        tm.set_year(Some(2069)).map_err(|err| err.to_string())?;
    }
    classify(&tm)
}

fn classify(tm: &BrokenDownTime) -> Result<Parsed, String> {
    let has_time = tm.hour().is_some() && tm.minute().is_some();
    let date = tm.to_date().ok();
    if let Some(timestamp) = tm.timestamp() {
        let offset = tm.offset().unwrap_or(Offset::UTC);
        return Ok(Parsed::Instant(DateTime::new(timestamp, offset)));
    }
    match (date, has_time) {
        (Some(_), true) => {
            let civil = tm.to_datetime().map_err(|err| err.to_string())?;
            Ok(match tm.offset() {
                Some(offset) => Parsed::Instant(
                    DateTime::from_civil(civil, offset)
                        .ok_or_else(|| "input is out of range".to_string())?,
                ),
                None => Parsed::Civil(civil),
            })
        }
        (Some(date), false) => Ok(Parsed::Date(date)),
        (None, true) => tm
            .to_time()
            .map(Parsed::Time)
            .map_err(|err| err.to_string()),
        (None, false) => Err("input is not enough for a date or a time".to_string()),
    }
}

/// A directive nushell reads itself, between the parts jiff reads.
#[derive(Clone, Copy)]
enum Special {
    /// `%Z`: skip a run of non-whitespace.
    ZoneName,
    /// `%z` and its colon forms; `permissive` (`%#z`) also reads `Z` and a bare hour.
    Offset { permissive: bool },
}

/// The format as jiff segments split at each [`Special`], and whether it reads a two-digit year.
struct Rewritten {
    segments: Vec<String>,
    specials: Vec<Special>,
    two_digit_year: bool,
}

fn rewrite_format(format: &str) -> Rewritten {
    let mut segments = Vec::new();
    let mut specials = Vec::new();
    let mut out = String::new();
    let mut two_digit_year = false;
    let mut special = |out: &mut String, kind: Special| {
        segments.push(std::mem::take(out));
        specials.push(kind);
    };
    let mut chars = format.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        if matches!(chars.peek(), Some('E' | 'O')) {
            chars.next();
        }
        // Offsets: `%z`, `%:z`, `%::z`, `%:::z` and `%#z`.
        let mut lookahead = chars.clone();
        let permissive = lookahead.next_if_eq(&'#').is_some();
        while lookahead.next_if_eq(&':').is_some() {}
        if lookahead.next_if_eq(&'z').is_some() {
            chars = lookahead;
            special(&mut out, Special::Offset { permissive });
            continue;
        }
        let expansion = match chars.peek() {
            Some('J') => "%Y%m%d",
            Some('Q') => "%H%M%S",
            Some('c') => "%a %b %e %H:%M:%S %Y",
            Some('x' | 'D') => {
                two_digit_year = true;
                "%m/%d/%y"
            }
            Some('X') => "%H:%M:%S",
            Some('r') => "%I:%M:%S %p",
            Some('+') => {
                chars.next();
                out.push_str("%Y-%m-%dT%H:%M:%S%.f");
                special(&mut out, Special::Offset { permissive: false });
                continue;
            }
            Some('v') => "%e-%b-%Y",
            Some('Z') => {
                chars.next();
                special(&mut out, Special::ZoneName);
                continue;
            }
            Some('y') => {
                two_digit_year = true;
                "%y"
            }
            Some('%') => "%%",
            _ => {
                out.push('%');
                continue;
            }
        };
        chars.next();
        out.push_str(expansion);
    }
    segments.push(out);
    Rewritten {
        segments,
        specials,
        two_digit_year,
    }
}

/// Parses `text` segment by segment, reading each [`Special`] between them, and merges what
/// the segments read.
fn parse_segments(text: &str, rewritten: &Rewritten) -> Result<BrokenDownTime, String> {
    let segments = &rewritten.segments;
    let last = segments.len() - 1;
    let mut tm = BrokenDownTime::default();
    let mut consumed = 0;
    for (i, segment) in segments.iter().enumerate() {
        let remaining = &text[consumed..];
        let (part, used) = if i == last {
            let part = BrokenDownTime::parse(segment, remaining).map_err(|err| err.to_string())?;
            (part, remaining.len())
        } else {
            BrokenDownTime::parse_prefix(segment, remaining).map_err(|err| err.to_string())?
        };
        merge(&mut tm, &part).map_err(|err| err.to_string())?;
        consumed += used;
        let Some(special) = rewritten.specials.get(i) else {
            continue;
        };
        let remaining = &text[consumed..];
        consumed += match special {
            Special::ZoneName => {
                remaining.len()
                    - remaining
                        .trim_start_matches(|c: char| !c.is_whitespace())
                        .len()
            }
            Special::Offset { permissive } => {
                let (seconds, used) = scan_offset(remaining, *permissive)
                    .ok_or_else(|| "input does not have a valid offset".to_string())?;
                let offset = Offset::from_seconds(seconds).map_err(|err| err.to_string())?;
                tm.set_offset(Some(offset));
                used
            }
        };
    }
    Ok(tm)
}

/// Reads an offset at the start of `s` as chrono's `%z` did: leading whitespace, a sign, two
/// hour digits, an optional colon or spaces, and two minute digits. `permissive` also reads `Z`
/// and an offset without minutes. Returns the offset in seconds and the bytes read.
fn scan_offset(s: &str, permissive: bool) -> Option<(i32, usize)> {
    let trimmed = s.trim_start();
    let start = s.len() - trimmed.len();
    if permissive && trimmed.starts_with(['Z', 'z']) {
        return Some((0, start + 1));
    }
    let sign = trimmed.chars().next()?;
    let negative = match sign {
        '+' => false,
        '-' | '\u{2212}' => true,
        _ => return None,
    };
    let rest = &trimmed[sign.len_utf8()..];
    let digits = |s: &str| -> Option<i32> {
        let d = s.get(..2)?;
        d.bytes()
            .all(|b| b.is_ascii_digit())
            .then(|| d.parse().ok())?
    };
    let hours = digits(rest)?;
    let after_hours = &rest[2..];
    let after_separator = after_hours.trim_start_matches(|c: char| c == ':' || c.is_whitespace());
    let (minutes, rest) = match digits(after_separator) {
        Some(minutes) if minutes < 60 => (minutes, &after_separator[2..]),
        Some(_) => return None,
        None if permissive => (0, after_hours),
        None => return None,
    };
    let seconds = hours * 3600 + minutes * 60;
    Some((
        if negative { -seconds } else { seconds },
        s.len() - rest.len(),
    ))
}

/// Copies every field `from` has into `into`.
fn merge(into: &mut BrokenDownTime, from: &BrokenDownTime) -> Result<(), jiff::Error> {
    macro_rules! copy {
        ($($get:ident => $set:ident),* $(,)?) => {
            $(if let Some(value) = from.$get() { into.$set(Some(value))?; })*
        };
    }
    copy!(
        year => set_year,
        month => set_month,
        day => set_day,
        day_of_year => set_day_of_year,
        iso_week_year => set_iso_week_year,
        iso_week => set_iso_week,
        sunday_based_week => set_sunday_based_week,
        monday_based_week => set_monday_based_week,
        hour => set_hour,
        minute => set_minute,
        second => set_second,
        subsec_nanosecond => set_subsec_nanosecond,
    );
    if let Some(offset) = from.offset() {
        into.set_offset(Some(offset));
    }
    if let Some(weekday) = from.weekday() {
        into.set_weekday(Some(weekday));
    }
    if let Some(meridiem) = from.meridiem() {
        into.set_meridiem(Some(meridiem));
    }
    if let Some(timestamp) = from.timestamp() {
        into.set_timestamp(Some(timestamp));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str, format: &str) -> String {
        match parse_with_format(text, format) {
            Ok(Parsed::Instant(date)) => format!("instant {}", date.to_rfc3339()),
            Ok(Parsed::Civil(civil)) => format!("civil {civil}"),
            Ok(Parsed::Date(date)) => format!("date {date}"),
            Ok(Parsed::Time(time)) => format!("time {time}"),
            Err(_) => "error".to_string(),
        }
    }

    #[test]
    fn reads_what_chrono_read() {
        assert_eq!(
            parse("16.11.1984 8:00 am +0000", "%d.%m.%Y %H:%M %P %z"),
            "instant 1984-11-16T08:00:00+00:00"
        );
        assert_eq!(
            parse("16.11.1984 8:00 am", "%d.%m.%Y %H:%M %P"),
            "civil 1984-11-16T08:00:00"
        );
        assert_eq!(parse("16.11.1984", "%d.%m.%Y"), "date 1984-11-16");
        assert_eq!(parse("08:30", "%H:%M"), "time 08:30:00");
        assert_eq!(
            parse("20250418_160432", "%J_%Q"),
            "civil 2025-04-18T16:04:32"
        );
        assert_eq!(
            parse("1744949072", "%s"),
            "instant 2025-04-18T04:04:32+00:00"
        );
        assert_eq!(
            parse("Fri Apr 18 16:04:32 2025", "%c"),
            "civil 2025-04-18T16:04:32"
        );
        assert_eq!(
            parse("2025-04-18T16:04:32.5+05:30", "%+"),
            "instant 2025-04-18T16:04:32.500+05:30"
        );
        assert_eq!(parse("2025-04-18", "%Y-%m"), "error");
    }

    #[test]
    fn two_digit_years_below_70_are_this_century() {
        assert_eq!(parse("69-01-01", "%y-%m-%d"), "date 2069-01-01");
        assert_eq!(parse("70-01-01", "%y-%m-%d"), "date 1970-01-01");
        assert_eq!(parse("04/18/69", "%D"), "date 2069-04-18");
    }

    #[test]
    fn offsets_read_with_or_without_a_colon() {
        for text in [
            "2021-02-27 21:55:40 +08:00",
            "2021-02-27 21:55:40 +0800",
            "2021-02-27 21:55:40 +08 00",
        ] {
            assert_eq!(
                parse(text, "%Y-%m-%d %H:%M:%S %z"),
                "instant 2021-02-27T21:55:40+08:00",
                "{text}"
            );
        }
        assert_eq!(
            parse("2021-02-27 21:55 Z", "%Y-%m-%d %H:%M %#z"),
            "instant 2021-02-27T21:55:00+00:00"
        );
        assert_eq!(
            parse("2021-02-27 21:55 +08", "%Y-%m-%d %H:%M %#z"),
            "instant 2021-02-27T21:55:00+08:00"
        );
        assert_eq!(parse("2021-02-27 21:55 Z", "%Y-%m-%d %H:%M %z"), "error");
    }

    #[test]
    fn zone_names_are_skipped() {
        assert_eq!(
            parse("2025-04-18 16:04 EST", "%Y-%m-%d %H:%M %Z"),
            "civil 2025-04-18T16:04:00"
        );
        assert_eq!(
            parse("2025-04-18 EST 16:04", "%Y-%m-%d %Z %H:%M"),
            "civil 2025-04-18T16:04:00"
        );
    }
}
