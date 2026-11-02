//! Reads a date or date-time written in one of the layouts people commonly use, for
//! `into datetime` and `detect type` when no format is given.
//!
//! jiff's strptime reads the fields. This module holds the layouts and the order to try them in,
//! rewrites that turn spelling variants into those layouts, and the rules for filling in what a
//! string leaves out: missing parts come from today, as in python-dateutil. With the `os`
//! feature, strings with relative words (`tomorrow 10am`, `3 days ago`, `next friday`) go to
//! parse_datetime, which reads GNU `date -d` syntax.

use chrono::{
    DateTime, Datelike, Days, FixedOffset, Local, LocalResult, Months, NaiveDate, NaiveDateTime,
    NaiveTime, TimeZone, Weekday,
};
use fancy_regex::Regex;
use jiff::fmt::strtime::{BrokenDownTime, Meridiem};
use std::{borrow::Cow, cell::LazyCell, sync::LazyLock};

/// Date layouts, tried in order as a prefix of the input. Two-digit years come before four-digit
/// ones so `04/18/25` and `18 Apr 25` are 2025 (jiff's `%Y` also reads two digits, as the year
/// 25), and month-first comes before day-first so `01/02/2025` is January 2; the day-first
/// reading is used when month-first doesn't make a date (`18/04/2025`). Month names are rewritten
/// to three letters first, so `%b` reads them all.
const DATES: &[&str] = &[
    "%m/%d/%y",
    "%d/%m/%y",
    "%m/%d/%Y",
    "%d/%m/%Y", //
    "%m-%d-%y",
    "%d-%m-%y",
    "%m-%d-%Y",
    "%d-%m-%Y", //
    "%m.%d.%y",
    "%d.%m.%y",
    "%m.%d.%Y",
    "%d.%m.%Y", //
    "%Y-%m-%d",
    "%Y/%m/%d",
    "%Y.%m.%d",
    "%G-W%V-%u",
    "%GW%V%u", //
    "%m%d%y",
    "%d%m%y",
    "%y%m%d", //
    "%d-%b-%y",
    "%d-%b-%Y",
    "%Y-%b-%d",
    "%b-%d-%y",
    "%b-%d-%Y",
    "%d%b%y",
    "%d%b%Y", //
    "%d %b %y",
    "%d %b %Y",
    "%d %b",
    "%b %d, %y",
    "%b %d, %Y",
    "%b %d %y",
    "%b %d %Y",
    "%b %d",
    "%b", //
    "%Y %b %d",
    "%Y %b", //
    "%m-%d",
    "%m/%d",
    "%Y-%m",
    "%Y-%j",
    "%Y",
];

/// Time layouts after a date, or alone (all but the last [`AFTER_DATE_ONLY`]). A layout counts
/// only when the rest of the string is an offset or nothing, so `4:04 PM` falls through `%H:%M`
/// to `%I:%M %p`. A space in a jiff layout also matches no space, so `%I:%M %p` reads `4:04PM`;
/// `%I%p` is still needed because [`prefix`] wants the separator right after the digits.
const TIMES: &[&str] = &[
    "%H:%M:%S%.f",
    "%H:%M",
    "%I:%M:%S%.f %p",
    "%I:%M %p",
    "%I %p",
    "%I%p",
    "%H%M%S",
    "%H%M",
    "%H",
];

/// The layouts at the end of [`TIMES`] that are tried only after a date: alone they would read
/// bare numbers (`1230`) as times.
const AFTER_DATE_ONLY: usize = 3;

/// Layouts read as the whole string: the year comes after the time (`date`, ctime, git log), or
/// the string starts with a literal.
const WHOLE: &[&str] = &[
    "%b %e %H:%M:%S%.f %Y %z",
    "%b %e %H:%M:%S%.f %Y %:z",
    "%b %e %I:%M:%S%.f %p %Y",
    "%b %e %H:%M:%S%.f %y",
    "%b %e %H:%M:%S%.f %Y",
    "%b %e %H:%M:%S%.f",
    "%d/%b/%Y:%H:%M:%S %z",
    "@%s",
];

/// Offsets after a time. `%Q` (an IANA name such as `America/New_York`) is tried only on text
/// with a `/`, since it would also read `am`.
const ZONES: &[&str] = &["%:z", "%z", "%Q"];

/// Rewrites from spelling variants to the layouts above, applied in order.
static REWRITES: LazyLock<Vec<(Regex, &str)>> = LazyLock::new(|| {
    [
        // one space between tokens
        (r"\s+", " "),
        (" ,", ","),
        // a zone name in parentheses, as in RFC 2822 comments: read like a bare zone name
        (r" ?\(([A-Z]{1,5})\)$", " $1"),
        // a.m. / p.m.
        (r"(?i)([\d ])([ap])\.?m\.?(\s|$)", "$1${2}M$3"),
        // 18th, 1st
        (r"(?i)(\d)(st|nd|rd|th)\b", "$1"),
        // 18 of April
        (r"(?i)\bof ", ""),
        // month names to three letters, with or without a dot: `Apr.`, `Sept.`
        (
            r"(?i)\b(jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec)(uary|ruary|ch|il|e|y|ust|t|tember|ober|ember)?(?:\.|\b)",
            "$1",
        ),
        // fraction digits past nanoseconds
        (r"(\.\d{9})\d+", "$1"),
        // '96
        (r"'(\d\d)\b", "$1"),
        // 2003 09 25, 09 25 2003; not 12 25 10:00, where the third number is the hour. This
        // rule, the next one and the basic-time rule match the next character instead of looking
        // ahead, because fancy_regex runs a pattern with a lookaround in its slower backtracking
        // engine.
        (r"^(\d{1,4}) (\d{1,2}) (\d{1,4})( |$)", "$1/$2/$3$4"),
        // 12 25, 12 25 10:00
        (r"^(\d{1,2}) (\d{1,2})( \d{1,2}:|$)", "$1/$2$3"),
        // lowercase ISO 8601 separator
        (r"(\d)t(\d)", "${1}T$2"),
        // T04:04:32
        (r"^T(\d)", "$1"),
        // basic time after an extended date: 2025-04-18T040432, 2025-04-18T040432Z
        (r"([T ])(\d\d)(\d\d)(\d\d)(\D|$)", "$1$2:$3:$4$5"),
        // comma fraction: 04:04:32,5
        (r"(:\d\d),(\d)", "$1.$2"),
        // empty fraction: 04:04:32.
        (r"(:\d\d)[.,]( |$|[+-]|[A-Za-z])", "$1$2"),
        // zero offset by name
        (r"(?i)(\d) ?(UTC|GMT|Z)$", "$1+00:00"),
        // ctime with a zero offset: ... 04:04:32 UTC 2025
        (r"(?i) (UTC|GMT)( \d{4})$", "$2 +00:00"),
        // +1:00, +01:0, +05, 10:00 +05
        (r"([+-])(\d):(\d\d)$", "${1}0$2:$3"),
        (r"([+-]\d\d):(\d)$", "$1:${2}0"),
        (r"(:\d\d(?:\.\d+)?) ?([+-]\d\d)$", "$1$2:00"),
    ]
    .into_iter()
    .map(|(pattern, to)| {
        let re = Regex::new(pattern).expect("date rewrite pattern should be valid");
        (re, to)
    })
    .collect()
});

/// A zone name at the end (`CET`, `CDT`, or ctime's `CDT 2025`), applied after [`REWRITES`].
/// [`normalize`] drops it, so the time is read as local time, unless it's a month name.
static ZONE_NAME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r" ([A-Z]{3,5})( \d{4})?$").expect("zone name pattern should be valid")
});

/// The month names the month rewrite in [`REWRITES`] shortens to.
const MONTHS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];

static WEEKDAY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"(?i)^(mon(day)?|tue(s|sday)?|wed(nesday)?|thu(rs|rsday)?|fri(day)?|sat(urday)?",
        r"|sun(day)?)\b\.?,? ?",
    ))
    .expect("weekday pattern should be valid")
});

/// GNU relative items, `@epoch` and `TZ="..."`: the only strings parse_datetime gets. That keeps
/// it away from what GNU reads differently: bare numbers (`2003` is 20:03 there), single
/// letters (military zones: `10h` is +08:00) and `03 25 Sep` (September 25 at 03:00).
#[cfg(feature = "os")]
static RELATIVE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"(?i)^@|^TZ=|\b(now|today|tomorrow|yesterday|ago|next|last|this|fortnights?|years?",
        r"|months?|weeks?|days?|hours?|minutes?|mins?|seconds?|secs?",
        r"|(mon|tue|wed|thu|fri|sat|sun)[a-z]*)\b",
    ))
    .expect("relative date pattern should be valid")
});

/// Read `input` as a date-time, or `None` if it isn't one.
pub(crate) fn parse_date_text(input: &str) -> Option<DateTime<FixedOffset>> {
    let input = input.trim();
    // jiff reads a leap second (`23:59:60`) as `:59`; nushell rejects leap seconds. The check
    // runs again after normalize, which writes basic-format seconds (`T235960`) as `:60`.
    if input.contains(":60") {
        return None;
    }
    if may_be_rfc2822(input)
        && let Ok(zoned) = jiff::fmt::rfc2822::parse(input)
    {
        return zoned_to_chrono(&zoned);
    }
    let (text, weekday) = normalize(input);
    if text.contains(":60") {
        return None;
    }
    if text.is_empty() {
        return weekday.and_then(|weekday| build(None, None, None, Some(weekday)));
    }
    if is_number(&text) {
        return None;
    }
    if let Some(date) = read_layouts(&text, weekday) {
        return Some(date);
    }
    // RFC 9557: 2025-04-18T04:04:32-05:00[America/Chicago]
    if input.ends_with(']') {
        return zoned_to_chrono(&input.parse::<jiff::Zoned>().ok()?);
    }
    parse_relative(input)
}

#[cfg(feature = "os")]
fn parse_relative(input: &str) -> Option<DateTime<FixedOffset>> {
    if !RELATIVE.is_match(input).unwrap_or(false) {
        return None;
    }
    zoned_to_chrono(&parse_datetime::parse_datetime(input).ok()?.into_zoned()?)
}

/// Relative dates are read by parse_datetime, which nu-command depends on only with the `os`
/// feature.
#[cfg(not(feature = "os"))]
fn parse_relative(_input: &str) -> Option<DateTime<FixedOffset>> {
    None
}

/// Whether `input` can be RFC 2822 (`Fri, 18 Apr 2025 04:04:32 +0530`): after an optional
/// weekday and comma, a one- or two-digit day and a space, and a time somewhere. Most strings
/// aren't, and skipping them saves building jiff's parse error.
fn may_be_rfc2822(input: &str) -> bool {
    let date = input
        .split_once(',')
        .map_or(input, |(_, date)| date)
        .trim_start();
    let day = date.bytes().take_while(u8::is_ascii_digit).count();
    (1..=2).contains(&day)
        && date
            .as_bytes()
            .get(day)
            .is_some_and(u8::is_ascii_whitespace)
        && input.contains(':')
}

/// A signed number, a decimal, or one or two digits stays a number, so `into datetime` can read
/// an integer as a timestamp. Unsigned integers of three or more digits can be years
/// (`2003`) or compact dates (`20250418`).
fn is_number(text: &str) -> bool {
    let unsigned = text.trim_start_matches(['+', '-']);
    let (whole, fraction) = unsigned.split_once(['.', ',']).unwrap_or((unsigned, ""));
    let digits = |part: &str| part.bytes().all(|c| c.is_ascii_digit() || c == b',');
    !unsigned.is_empty()
        && digits(whole)
        && digits(fraction)
        && (unsigned.len() < text.len() || unsigned.len() != whole.len() || whole.len() <= 2)
}

/// Trim, rewrite spelling variants, and take off a leading weekday name (returned separately).
fn normalize(input: &str) -> (String, Option<Weekday>) {
    let trimmed = input.trim_matches(['\'', '"']);
    let mut text = expand_basic(trimmed).unwrap_or_else(|| trimmed.to_string());
    for (re, to) in REWRITES.iter() {
        if let Ok(Cow::Owned(rewritten)) = re.try_replacen(&text, 0, *to) {
            text = rewritten;
        }
    }
    if let Ok(Some(caps)) = ZONE_NAME.captures(&text)
        && let (Some(found), Some(name)) = (caps.get(0), caps.get(1))
        && !MONTHS
            .iter()
            .any(|month| name.as_str().eq_ignore_ascii_case(month))
    {
        let year = caps.get(2).map_or("", |year| year.as_str()).to_string();
        text.replace_range(found.range(), &year);
    }
    let mut weekday = None;
    if let Ok(Some(found)) = WEEKDAY.find(&text) {
        weekday = found.as_str().get(..3).and_then(|w| w.parse().ok());
        text = text
            .get(found.end()..)
            .unwrap_or_default()
            .trim()
            .to_string();
    }
    (text, weekday)
}

/// ISO 8601 basic format to extended: `20250418` to `2025-04-18`, `20250418T0404` and
/// `202504180404` to `2025-04-18T04:04`.
fn expand_basic(text: &str) -> Option<String> {
    let digits = text.bytes().take_while(u8::is_ascii_digit).count();
    let after = text.get(digits..)?;
    let joined_time = (digits == 12 || digits == 14)
        && (after.is_empty() || after.starts_with(['.', ',', '+', '-', 'Z', 'z']));
    if digits != 8 && !joined_time {
        return None;
    }
    let mut out = format!("{}-{}-{}", text.get(..4)?, text.get(4..6)?, text.get(6..8)?);
    let rest = text.get(8..)?;
    let Some(time) = rest
        .strip_prefix(['T', 't'])
        .or(joined_time.then_some(rest))
    else {
        out.push_str(rest);
        return Some(out);
    };
    let n = time.bytes().take_while(u8::is_ascii_digit).count();
    out.push('T');
    match n {
        // 20080227T21:26:01 already has an extended time
        2 if time.get(2..)?.starts_with(':') => out.push_str(time.get(..2)?),
        2 => {
            out.push_str(time.get(..2)?);
            out.push_str(":00");
        }
        4 | 6 => {
            let pairs: Vec<&str> = (0..n)
                .step_by(2)
                .filter_map(|i| time.get(i..i + 2))
                .collect();
            out.push_str(&pairs.join(":"));
        }
        _ => out.push_str(time.get(..n)?),
    }
    out.push_str(time.get(n..)?);
    Some(out)
}

/// `layout` as a prefix of `text`: the fields and the text after them.
///
/// A layout whose start can't match is skipped before jiff sees it, because a failed jiff parse
/// builds an error value and most strings fail most layouts: a month name needs a letter, a
/// number needs its digits (three or four for a year, one or two otherwise) followed by the
/// layout's separator, numbers run together (`%m%d%y`, `%H%M`) need two digits each, and every
/// separator in the layout has to be somewhere in the text.
fn prefix<'a>(layout: &str, text: &'a str) -> Option<(BrokenDownTime, &'a str)> {
    let lead = text.bytes().take_while(u8::is_ascii_digit).count();
    let spec = layout.as_bytes().get(1).copied();
    let sep = layout.as_bytes().get(2).copied().filter(|c| *c != b'%');
    let fields_run_together = layout.as_bytes().get(2) == Some(&b'%');
    match spec {
        Some(b'b') if lead > 0 => return None,
        // numbers run together take two digits each: jiff also reads one, so `11111` would be
        // 2001-11-11 and `2025-04-18T123` would be 12:03
        Some(_)
            if fields_run_together
                && layout.bytes().all(|c| c == b'%' || b"mdyHMS".contains(&c))
                && lead != layout.len() =>
        {
            return None;
        }
        Some(b'Y' | b'G') if !fields_run_together && !(3..=4).contains(&lead) => return None,
        Some(b'm' | b'd' | b'y' | b'H' | b'I')
            if !fields_run_together && !(1..=2).contains(&lead) =>
        {
            return None;
        }
        _ => {}
    }
    if !layout.starts_with('%') && !text.starts_with(layout.get(..1)?) {
        return None;
    }
    if let Some(sep) = sep.filter(|_| layout.starts_with('%') && lead > 0)
        && text.len() > lead
        && text.as_bytes().get(lead) != Some(&sep)
    {
        return None;
    }
    let bytes = layout.as_bytes();
    let mut i = 0;
    while let Some(&c) = bytes.get(i) {
        if c == b'%' {
            i += if bytes.get(i + 1) == Some(&b'.') {
                3
            } else {
                2
            };
            continue;
        }
        if !c.is_ascii_alphanumeric() && c != b' ' && !text.as_bytes().contains(&c) {
            return None;
        }
        i += 1;
    }
    let (fields, end) = BrokenDownTime::parse_prefix(layout, text).ok()?;
    Some((fields, text.get(end..)?))
}

/// The time and offset in `rest`, after a date (`T`, a space or a comma first) or alone.
/// After a date, an empty `rest` is a date alone.
fn time_and_zone(
    rest: &str,
    after_date: bool,
) -> Option<(Option<BrokenDownTime>, Option<BrokenDownTime>)> {
    if rest.is_empty() {
        return after_date.then_some((None, None));
    }
    let rest = match rest.strip_prefix(['T', ' ', ',']) {
        Some(rest) => rest.trim_start(),
        None if after_date => return None,
        None => rest,
    };
    let times = if after_date {
        TIMES
    } else {
        TIMES.get(..TIMES.len() - AFTER_DATE_ONLY)?
    };
    times.iter().find_map(|layout| {
        let (time, after) = prefix(layout, rest)?;
        let after = after.trim_start();
        if after.is_empty() {
            return Some((Some(time), None));
        }
        let zones = if after.contains('/') {
            ZONES
        } else {
            ZONES.get(..2)?
        };
        let (zone, _) = zones
            .iter()
            .find_map(|z| prefix(z, after).filter(|(_, left)| left.is_empty()))?;
        Some((Some(time), Some(zone)))
    })
}

/// The first reading of `text` in layout order that makes a valid date-time. A reading that
/// doesn't (`2025-02-30`, or `18/04/2025` month-first) falls through to the next layout.
fn read_layouts(text: &str, weekday: Option<Weekday>) -> Option<DateTime<FixedOffset>> {
    let whole = |layout: &str| {
        prefix(layout, text)
            .filter(|(_, rest)| rest.is_empty())
            .map(|(fields, _)| fields)
    };
    // `Apr 2025`: read before `%b %d %Y`, which would take it as April 20 of the year 25
    let month_year = text.split_once(' ').is_some_and(|(month, year)| {
        month
            .trim_end_matches([',', '.'])
            .bytes()
            .all(|c| c.is_ascii_alphabetic())
            && (3..=4).contains(&year.len())
            && year.bytes().all(|c| c.is_ascii_digit())
    });
    if month_year
        && let Some(date) = whole("%b %Y").and_then(|f| build(Some(&f), None, None, weekday))
    {
        return Some(date);
    }
    for layout in WHOLE {
        if let Some(fields) = whole(layout) {
            let zone = fields.offset().is_some().then_some(&fields);
            if let Some(date) = build(Some(&fields), Some(&fields), zone, weekday) {
                return Some(date);
            }
        }
    }
    for layout in DATES {
        let Some((date, rest)) = prefix(layout, text) else {
            continue;
        };
        let Some((time, zone)) = time_and_zone(rest, true) else {
            continue;
        };
        if let Some(date) = build(Some(&date), time.as_ref(), zone.as_ref(), weekday) {
            return Some(date);
        }
    }
    let (time, zone) = time_and_zone(text, false)?;
    build(None, time.as_ref(), zone.as_ref(), weekday)
}

/// Fill what the string left out from today, move to `weekday` when there's no day, and apply
/// the offset, the IANA zone, or local time.
fn build(
    date: Option<&BrokenDownTime>,
    time: Option<&BrokenDownTime>,
    zone: Option<&BrokenDownTime>,
    weekday: Option<Weekday>,
) -> Option<DateTime<FixedOffset>> {
    if let Some(timestamp) = date.and_then(|d| d.timestamp()) {
        return DateTime::from_timestamp(timestamp.as_second(), 0).map(|d| d.fixed_offset());
    }
    // read the clock only when the string leaves out part of the date
    let today = LazyCell::new(|| Local::now().date_naive());
    let mut day_date = match date {
        Some(d) if d.iso_week().is_some() || d.day_of_year().is_some() => {
            let d = d.to_date().ok()?;
            let month = u32::try_from(d.month()).ok()?;
            let day = u32::try_from(d.day()).ok()?;
            NaiveDate::from_ymd_opt(d.year().into(), month, day)?
        }
        _ => {
            let year = date
                .and_then(|d| d.year())
                .map_or_else(|| today.year(), i32::from);
            let month = match date.and_then(|d| d.month()) {
                Some(month) => u32::try_from(month).ok()?,
                None => today.month(),
            };
            let day = match date.and_then(|d| d.day()) {
                Some(day) => u32::try_from(day).ok()?,
                // today's day, kept within the month: `Feb 2025` on the 30th is February 28
                None => {
                    let first = NaiveDate::from_ymd_opt(year, month, 1)?;
                    let last = first.checked_add_months(Months::new(1))?.pred_opt()?;
                    today.day().min(last.day())
                }
            };
            NaiveDate::from_ymd_opt(year, month, day)?
        }
    };
    if let (Some(weekday), None) = (weekday, date.and_then(|d| d.day())) {
        let ahead =
            (7 + weekday.num_days_from_monday() - day_date.weekday().num_days_from_monday()) % 7;
        day_date = day_date.checked_add_days(Days::new(ahead.into()))?;
    }
    let field = |get: fn(&BrokenDownTime) -> Option<i8>| {
        time.and_then(get)
            .map_or(Some(0), |v| u32::try_from(v).ok())
    };
    let mut hour = field(BrokenDownTime::hour)?;
    match time.and_then(|t| t.meridiem()) {
        Some(Meridiem::PM) if hour < 12 => hour += 12,
        Some(Meridiem::AM) if hour == 12 => hour = 0,
        _ => {}
    }
    let (minute, second) = (
        field(BrokenDownTime::minute)?,
        field(BrokenDownTime::second)?,
    );
    let nanosecond = match time.and_then(|t| t.subsec_nanosecond()) {
        Some(nanosecond) => u32::try_from(nanosecond).ok()?,
        None => 0,
    };
    let naive = day_date.and_time(NaiveTime::from_hms_nano_opt(
        hour, minute, second, nanosecond,
    )?);
    if let Some(name) = zone.and_then(|z| z.iana_time_zone()) {
        let civil = jiff::civil::DateTime::new(
            i16::try_from(naive.year()).ok()?,
            i8::try_from(naive.month()).ok()?,
            i8::try_from(naive.day()).ok()?,
            i8::try_from(hour).ok()?,
            i8::try_from(minute).ok()?,
            i8::try_from(second).ok()?,
            i32::try_from(nanosecond).ok()?,
        )
        .ok()?;
        let zone = jiff::tz::TimeZone::get(name).ok()?;
        return zoned_to_chrono(&civil.to_zoned(zone).ok()?);
    }
    let offset = match zone.and_then(|z| z.offset()) {
        Some(offset) => Some(FixedOffset::east_opt(offset.seconds())?),
        None => None,
    };
    with_offset(naive, offset)
}

/// `naive` at `offset`, or in local time. Local time follows jiff's rule for IANA zones, so both
/// read a wall-clock time the same way: when the clock goes back and repeats it, the earlier
/// offset; when the clock jumps ahead and skips it, the offset from before the jump, which moves
/// it forward by the jump (02:30 on the spring-forward day reads as 03:30).
fn with_offset(naive: NaiveDateTime, offset: Option<FixedOffset>) -> Option<DateTime<FixedOffset>> {
    if let Some(offset) = offset {
        return offset.from_local_datetime(&naive).single();
    }
    match Local.from_local_datetime(&naive) {
        LocalResult::Single(date) => Some(date.fixed_offset()),
        // chrono lists the later reading first
        LocalResult::Ambiguous(first, second) => Some(first.min(second).fixed_offset()),
        LocalResult::None => {
            // a day earlier is before the jump
            let before = Local.offset_from_utc_datetime(&naive.checked_sub_days(Days::new(1))?);
            let date = before.from_local_datetime(&naive).single()?;
            Some(date.with_timezone(&Local).fixed_offset())
        }
    }
}

/// jiff's `Zoned` as nushell's date value, keeping the instant and the offset.
fn zoned_to_chrono(zoned: &jiff::Zoned) -> Option<DateTime<FixedOffset>> {
    let offset = FixedOffset::east_opt(zoned.offset().seconds())?;
    let timestamp = zoned.timestamp();
    let (seconds, nanoseconds) = match timestamp.subsec_nanosecond() {
        n if n < 0 => (timestamp.as_second() - 1, n + 1_000_000_000),
        n => (timestamp.as_second(), n),
    };
    let utc = DateTime::from_timestamp(seconds, u32::try_from(nanoseconds).ok()?)?;
    Some(utc.with_timezone(&offset))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    /// The wall-clock reading, without the offset, which depends on the local zone.
    fn wall(input: &str) -> Option<String> {
        parse_date_text(input).map(|d| d.naive_local().format("%Y-%m-%d %H:%M:%S%.f").to_string())
    }

    /// The reading with its offset, for inputs that carry one.
    fn exact(input: &str) -> Option<String> {
        parse_date_text(input).map(|d| d.to_rfc3339())
    }

    #[test]
    fn iso_8601_shapes() {
        for (input, expected) in [
            ("2025-04-18", "2025-04-18 00:00:00"),
            ("2025-4-8", "2025-04-08 00:00:00"),
            ("2025-04-18 04:04:32", "2025-04-18 04:04:32"),
            ("2025-04-18T04", "2025-04-18 04:00:00"),
            ("2025-04-18t04:04:32,5", "2025-04-18 04:04:32.500"),
            ("2025-04-18 04:04:32.", "2025-04-18 04:04:32"),
            (
                "2025-04-18 04:04:32.1234567891",
                "2025-04-18 04:04:32.123456789",
            ),
            ("20250418", "2025-04-18 00:00:00"),
            ("20250418T040432", "2025-04-18 04:04:32"),
            ("20250418040432", "2025-04-18 04:04:32"),
            ("2025-04-18T040432", "2025-04-18 04:04:32"),
            (
                "20080227T21:26:01.123456789",
                "2008-02-27 21:26:01.123456789",
            ),
            ("2025-04-18T1230", "2025-04-18 12:30:00"),
            ("01/02/2025 1230", "2025-01-02 12:30:00"),
        ] {
            assert_eq!(wall(input).as_deref(), Some(expected), "{input}");
        }
        // a year and month take today's day, kept within the month
        assert!(wall("2025-04").is_some_and(|d| d.starts_with("2025-04-")));
        assert!(wall("Feb 2025").is_some_and(|d| d.starts_with("2025-02-")));
    }

    #[test]
    fn offsets_keep_their_minutes() {
        for (input, expected) in [
            ("2025-04-18 04:04:32 +0530", "2025-04-18T04:04:32+05:30"),
            ("2025-04-18T04:04:32-0330", "2025-04-18T04:04:32-03:30"),
            ("2022-01-01T00:00:00+01", "2022-01-01T00:00:00+01:00"),
            ("2022-01-01T00:00:00+1:00", "2022-01-01T00:00:00+01:00"),
            ("2025-04-18 04:04:32 UTC", "2025-04-18T04:04:32+00:00"),
            ("20250418T040432Z", "2025-04-18T04:04:32+00:00"),
            (
                "Fri, 18 Apr 2025 04:04:32 +0530",
                "2025-04-18T04:04:32+05:30",
            ),
            ("Fri, 18 Apr 2025 04:04:32 GMT", "2025-04-18T04:04:32+00:00"),
            ("Fri, 18 Apr 2025 04:04:32 PDT", "2025-04-18T04:04:32-07:00"),
            ("2025-04-18 10:00 +05", "2025-04-18T10:00:00+05:00"),
            ("2025-04-18 10:00 +0100 (CET)", "2025-04-18T10:00:00+01:00"),
            ("2025-04-18 10:00 (UTC)", "2025-04-18T10:00:00+00:00"),
            (
                "Fri Apr 18 04:04:32 2025 -0700",
                "2025-04-18T04:04:32-07:00",
            ),
            ("Fri Apr 18 04:04:32 UTC 2025", "2025-04-18T04:04:32+00:00"),
            ("18/Apr/2025:04:04:32 +0530", "2025-04-18T04:04:32+05:30"),
            ("@1745000000", "2025-04-18T18:13:20+00:00"),
        ] {
            assert_eq!(exact(input).as_deref(), Some(expected), "{input}");
        }
    }

    #[test]
    fn iana_zones() {
        assert_eq!(
            exact("2025-04-18T04:04:32[America/Chicago]").as_deref(),
            Some("2025-04-18T04:04:32-05:00")
        );
        assert_eq!(
            exact("2025-04-18T04:04:32-05:00[America/Chicago]").as_deref(),
            Some("2025-04-18T04:04:32-05:00")
        );
        assert_eq!(
            exact("2025-04-18 04:04:32 America/New_York").as_deref(),
            Some("2025-04-18T04:04:32-04:00")
        );
    }

    #[test]
    fn numeric_dates_read_month_first_then_day_first() {
        for (input, expected) in [
            ("04/18/2025", "2025-04-18"),
            ("18/04/2025", "2025-04-18"),
            ("01/02/2025", "2025-01-02"),
            ("04/18/25", "2025-04-18"),
            ("09-25-2003", "2003-09-25"),
            ("18.04.2025", "2025-04-18"),
            ("10.09.2003", "2003-10-09"),
            ("2025/04/18", "2025-04-18"),
            ("2025.04.18", "2025-04-18"),
            ("2003 09 25", "2003-09-25"),
            ("09 25 2003", "2003-09-25"),
            ("250418", "2018-04-25"),
            ("2025-W16-5", "2025-04-18"),
            ("2025-108", "2025-04-18"),
        ] {
            let got = wall(input);
            assert_eq!(got.as_deref().map(|s| &s[..10]), Some(expected), "{input}");
        }
    }

    #[test]
    fn month_names_and_times() {
        for (input, expected) in [
            ("April 18, 2025", "2025-04-18 00:00:00"),
            ("18 Apr 2025", "2025-04-18 00:00:00"),
            ("18-Apr-2025 04:04:32.123", "2025-04-18 04:04:32.123"),
            ("2025-Apr-18", "2025-04-18 00:00:00"),
            ("13NOV2017", "2017-11-13 00:00:00"),
            ("Sept 18 2025", "2025-09-18 00:00:00"),
            ("Friday, April 18, 2025", "2025-04-18 00:00:00"),
            ("18th of April 2025", "2025-04-18 00:00:00"),
            ("April 18th, 2025", "2025-04-18 00:00:00"),
            ("Wed, July 10, '96", "1996-07-10 00:00:00"),
            ("Fri Apr 18 04:04:32 2025", "2025-04-18 04:04:32"),
            ("Fri Apr 18 04:04:32 PM CDT 2025", "2025-04-18 16:04:32"),
            ("04/18/2025 4:04 PM", "2025-04-18 16:04:00"),
            ("2025-04-18 12:00 AM", "2025-04-18 00:00:00"),
            ("16.11.1984 8:00 am", "1984-11-16 08:00:00"),
            // two-digit years after a month name
            ("18 Apr 25", "2025-04-18 00:00:00"),
            ("Apr 18 25", "2025-04-18 00:00:00"),
            ("Apr-18-25", "2025-04-18 00:00:00"),
            ("13NOV17", "2017-11-13 00:00:00"),
            ("Fri Apr 18 04:04:32 25", "2025-04-18 04:04:32"),
            // upper case, and a dot after the abbreviation
            ("18 APR 2025", "2025-04-18 00:00:00"),
            ("Apr. 18, 2025", "2025-04-18 00:00:00"),
            ("Sept. 18, 2025", "2025-09-18 00:00:00"),
            ("Dec. 25, 2025 10:00 AM", "2025-12-25 10:00:00"),
            ("2025 Dec 12", "2025-12-12 00:00:00"),
        ] {
            assert_eq!(wall(input).as_deref(), Some(expected), "{input}");
        }
    }

    #[test]
    fn missing_parts_come_from_today() {
        let today = Local::now().date_naive();
        let at = |h, m| {
            today
                .and_hms_opt(h, m, 0)
                .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
        };
        assert_eq!(wall("10:36"), at(10, 36));
        assert_eq!(wall("4 PM"), at(16, 0));
        assert_eq!(wall("10:00a.m."), at(10, 0));
        assert_eq!(wall("12am"), at(0, 0));
        let friday = parse_date_text("Friday").map(|d| d.date_naive());
        assert_eq!(friday.map(|d| d.weekday()), Some(Weekday::Fri));
        assert!(friday.is_some_and(|d| d >= today && d < today + Days::new(7)));
        assert_eq!(wall("4:04PM"), at(16, 4));
        for (input, month, day) in [("April 18", 4, 18), ("18 Apr", 4, 18), ("18 APR", 4, 18)] {
            let date = parse_date_text(input).map(|d| d.date_naive());
            assert_eq!(
                date.map(|d| (d.year(), d.month(), d.day())),
                Some((today.year(), month, day)),
                "{input}"
            );
        }
        assert_eq!(
            parse_date_text("12 25 10:00").map(|d| (d.month(), d.day(), d.hour())),
            Some((12, 25, 10))
        );
        // a year and month
        for (input, month) in [("2025 Dec", 12), ("2025 MAY", 5)] {
            let date = parse_date_text(input).map(|d| d.date_naive());
            assert_eq!(
                date.map(|d| (d.year(), d.month())),
                Some((2025, month)),
                "{input}"
            );
        }
    }

    #[test]
    fn not_dates() {
        for input in [
            "",
            "12",
            "3.14",
            "10,000",
            "-0500",
            "2025-02-30",
            "2025-13-01",
            "2016-12-31T23:59:60Z",
            "2016-12-31T235960Z",
            "20161231T235960",
            // five digits are a timestamp, not a compact date
            "11111",
            "12251",
            // a basic time needs two digits a field
            "2025-04-18T123",
            "2025-04-18T12345",
            "decimals",
            "7e-05",
            "(1 + (2 * 3))",
            // malformed input that made the previous parser (dtparse 2.0.1) panic
            "Jan-",
            "Jan-01-",
            "5h-",
            "10:00 +",
            "10:00 -03:",
            "0:0:.",
            "263660314029650025864127edc124c2",
        ] {
            assert_eq!(parse_date_text(input), None, "{input:?}");
        }
    }

    #[test]
    fn leap_years_follow_the_gregorian_rule() {
        assert_eq!(wall("2000-02-29").as_deref(), Some("2000-02-29 00:00:00"));
        assert_eq!(wall("1900-02-29"), None);
    }

    #[test]
    fn a_bare_dot_before_the_zone_is_an_empty_fraction() {
        assert_eq!(
            exact("2025-04-18T04:04:32.Z").as_deref(),
            Some("2025-04-18T04:04:32+00:00")
        );
    }

    #[cfg(feature = "os")]
    #[test]
    fn relative_dates() {
        let today = Local::now().date_naive();
        let date = |input| parse_date_text(input).map(|d| d.date_naive());
        assert_eq!(date("tomorrow"), today.checked_add_days(Days::new(1)));
        assert_eq!(date("3 days ago"), today.checked_sub_days(Days::new(3)));
        assert_eq!(
            parse_date_text("tomorrow 10am").map(|d| (d.date_naive(), d.hour())),
            today.checked_add_days(Days::new(1)).map(|d| (d, 10))
        );
        assert_eq!(
            exact("TZ=\"Europe/Paris\" 2025-04-18 04:04").as_deref(),
            Some("2025-04-18T04:04:00+02:00")
        );
    }
}
