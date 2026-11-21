//! `strftime` formatting for [`DateTime`], with the specifier meanings nushell has always had.
//!
//! jiff's `strftime` reads most specifiers the same way chrono did. This module rewrites the
//! format string first, so the rest print as before:
//!
//! - `%c`, `%x`, `%X` and `%r` expand from the locale's own formats, and `%E`/`%O` modifiers are
//!   dropped, since no locale table here has an alternative representation.
//! - `%a %A %b %B %h %p %P %v` use the locale's names, and `%.f %.3f %.6f %.9f` its decimal point.
//! - `%f` is 9 digits, `%.f` the shortest of 0, 3, 6 or 9, `%Z` the offset, `%:::z` whole hours,
//!   `%+` RFC 3339, and nushell's own `%J` and `%Q` are `%Y%m%d` and `%H%M%S`.
//!
//! Anything else goes to jiff unchanged, so jiff-only specifiers such as `%N` and `%q` work too.
//!
//! jiff's dates stop at years -9999 and 9999. For a year past them, jiff formats a date whole
//! 400-year cycles closer, which has the same calendar and weekdays, and this module writes the
//! year itself.

use super::{DateParts, DateTime, OffsetStyle, SecondsFormat, write_fraction, write_offset};
use jiff::{civil, fmt::strtime::BrokenDownTime};
use pure_rust_locales::{Locale, locale_match};
use std::fmt;

/// English short weekday names, Sunday first.
pub(super) const SHORT_WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

/// English short month names, January first.
pub(super) const SHORT_MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// A format string the date can't be written with, such as one with an unknown specifier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DateFormatError {
    format: String,
}

impl DateFormatError {
    /// The format string that failed.
    pub fn format(&self) -> &str {
        &self.format
    }
}

impl fmt::Display for DateFormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid date format string {:?}", self.format)
    }
}

impl std::error::Error for DateFormatError {}

pub(super) fn format(
    date: DateTime,
    format: &str,
    locale: Locale,
) -> Result<String, DateFormatError> {
    let error = || DateFormatError {
        format: format.to_string(),
    };
    let parts = date.parts();
    let proxy_year = if (-9999..=9999).contains(&parts.year) {
        parts.year
    } else {
        2000 + (parts.year - 2000).rem_euclid(400)
    };
    let proxy = civil::DateTime::new(
        proxy_year as i16,
        parts.month,
        parts.day,
        parts.hour,
        parts.minute,
        parts.second,
        parts.nanosecond,
    )
    .map_err(|_| error())?;
    let expanded = expand_locale_formats(format, locale);
    let jiff_format = to_jiff_format(date, parts, proxy, &expanded, locale);
    let mut fields = BrokenDownTime::from(proxy);
    fields.set_offset(Some(date.offset()));
    fields.to_string(&jiff_format).map_err(|_| error())
}

/// Expands `%c`, `%x`, `%X` and `%r` from `locale`, dropping `%E`/`%O` modifiers before and
/// after, since the locale's own formats can carry them (`th_TH` has `%Ey` in `%x`). A locale
/// format can name another (`en_US` has `%r` in `%c`), so expansion repeats a few times.
pub(super) fn expand_locale_formats(format: &str, locale: Locale) -> String {
    let mut current = strip_alternative_modifiers(format);
    for _ in 0..3 {
        let next = strip_alternative_modifiers(&expand_once(&current, locale));
        if next == current {
            break;
        }
        current = next;
    }
    current
}

fn expand_once(format: &str, locale: Locale) -> String {
    let mut out = String::with_capacity(format.len());
    let mut chars = format.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        let expansion = match chars.peek() {
            Some('%') => {
                chars.next();
                out.push_str("%%");
                continue;
            }
            Some('x') => locale_or(locale_match!(locale => LC_TIME::D_FMT), "%m/%d/%y"),
            Some('X') => locale_or(locale_match!(locale => LC_TIME::T_FMT), "%H:%M:%S"),
            Some('c') => locale_or(
                locale_match!(locale => LC_TIME::D_T_FMT),
                "%a %b %e %H:%M:%S %Y",
            ),
            // A locale can have no am/pm format at all (`de_DE`, `fr_FR`, `fa_IR`); its plain
            // time format is used then.
            Some('r') => locale_or(
                locale_match!(locale => LC_TIME::T_FMT_AMPM),
                locale_or(locale_match!(locale => LC_TIME::T_FMT), "%I:%M:%S %p"),
            ),
            _ => {
                out.push('%');
                continue;
            }
        };
        chars.next();
        out.push_str(expansion);
    }
    out
}

fn locale_or<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.is_empty() { fallback } else { value }
}

/// `%Ey` becomes `%y` and `%Od` becomes `%d`: the specifier stays, the alternative form goes.
fn strip_alternative_modifiers(format: &str) -> String {
    let mut out = String::with_capacity(format.len());
    let mut chars = format.chars().peekable();
    while let Some(c) = chars.next() {
        out.push(c);
        if c != '%' {
            continue;
        }
        match chars.peek() {
            Some('%') => {
                chars.next();
                out.push('%');
            }
            Some('E' | 'O') => {
                chars.next();
            }
            _ => {}
        }
    }
    out
}

/// One `%` directive: an optional flag, width digits, colons and dot, then the specifier.
struct Directive<'a> {
    text: &'a str,
    flag: Option<char>,
    width: Option<u32>,
    colons: usize,
    dot: bool,
    spec: Option<char>,
}

fn read_directive(format: &str) -> Directive<'_> {
    // `format` starts at the `%`.
    let bytes = format.as_bytes();
    let mut i = 1;
    let flag = match bytes.get(i) {
        Some(&b) if matches!(b, b'-' | b'_' | b'0' | b'^' | b'#') => {
            i += 1;
            Some(b as char)
        }
        _ => None,
    };
    let width_start = i;
    while bytes.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
    }
    let width = format[width_start..i].parse().ok();
    let colons_start = i;
    while bytes.get(i) == Some(&b':') {
        i += 1;
    }
    let colons = i - colons_start;
    let dot = bytes.get(i) == Some(&b'.');
    if dot {
        i += 1;
    }
    // A specifier after `.` can carry width digits too: `%.3f`.
    let width = if dot {
        let start = i;
        while bytes.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        format[start..i].parse().ok().or(width)
    } else {
        width
    };
    let spec = format[i..].chars().next();
    let end = i + spec.map_or(0, char::len_utf8);
    Directive {
        text: &format[..end],
        flag,
        width,
        colons,
        dot,
        spec,
    }
}

/// Rewrites `format` into a jiff format string, putting the text of the specifiers nushell
/// renders itself in as escaped literals.
fn to_jiff_format(
    date: DateTime,
    parts: DateParts,
    proxy: civil::DateTime,
    format: &str,
    locale: Locale,
) -> String {
    let mut out = String::with_capacity(format.len() * 2);
    let mut rest = format;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let directive = read_directive(&rest[start..]);
        rest = &rest[start + directive.text.len()..];
        let literal = |text: &str, out: &mut String| out.push_str(&text.replace('%', "%%"));
        let upper = |text: &str| {
            if directive.flag == Some('^') {
                text.to_uppercase()
            } else {
                text.to_string()
            }
        };
        let weekday = parts.weekday as usize;
        let month = parts.month as usize - 1;
        let pm = parts.hour >= 12;
        let plain = directive.colons == 0 && !directive.dot;
        // Without a flag or width, numbers print as chrono printed them; with one, they follow
        // jiff's padding rules.
        let bare = plain && directive.flag.is_none() && directive.width.is_none();
        let year = parts.year;
        let iso_year = i32::from(proxy.iso_week_date().year()) + (year - i32::from(proxy.year()));
        let padded = |value: i32, default_width: usize| pad(value, &directive, default_width);
        match directive.spec {
            Some('Y') if bare => literal(&four_digit_year(year), &mut out),
            Some('G') if bare => literal(&four_digit_year(iso_year), &mut out),
            Some('C') if bare => literal(&two_digits(year.div_euclid(100)), &mut out),
            Some('y') if bare => literal(&two_digits(year.rem_euclid(100)), &mut out),
            Some('g') if bare => literal(&two_digits(iso_year.rem_euclid(100)), &mut out),
            Some('Y') if plain => literal(&padded(year, 4), &mut out),
            Some('G') if plain => literal(&padded(iso_year, 4), &mut out),
            Some('C') if plain => literal(&padded(year.div_euclid(100), 2), &mut out),
            Some('y') if plain => literal(&padded(year.rem_euclid(100), 2), &mut out),
            Some('g') if plain => literal(&padded(iso_year.rem_euclid(100), 2), &mut out),
            Some('D') if bare => {
                out.push_str("%m/%d/");
                literal(&two_digits(year.rem_euclid(100)), &mut out);
            }
            Some('F') if bare => {
                literal(&four_digit_year(year), &mut out);
                out.push_str("-%m-%d");
            }
            // chrono's Unix seconds round down, so a time before 1970 with a fraction gives the
            // earlier second.
            Some('s') if bare => literal(&date.seconds.to_string(), &mut out),
            Some('%') if plain => out.push_str("%%"),
            Some('a') if plain => literal(
                &upper(locale_match!(locale => LC_TIME::ABDAY)[weekday]),
                &mut out,
            ),
            Some('A') if plain => literal(
                &upper(locale_match!(locale => LC_TIME::DAY)[weekday]),
                &mut out,
            ),
            Some('b' | 'h') if plain => literal(
                &upper(locale_match!(locale => LC_TIME::ABMON)[month]),
                &mut out,
            ),
            Some('B') if plain => literal(
                &upper(locale_match!(locale => LC_TIME::MON)[month]),
                &mut out,
            ),
            Some('p') if plain => {
                literal(&upper(am_pm(locale, pm)), &mut out);
            }
            Some('P') if plain => literal(&am_pm(locale, pm).to_lowercase(), &mut out),
            Some('v') if plain => {
                out.push_str("%e-");
                literal(locale_match!(locale => LC_TIME::ABMON)[month], &mut out);
                out.push('-');
                literal(&four_digit_year(year), &mut out);
            }
            Some('J') if plain => out.push_str("%Y%m%d"),
            Some('Q') if plain => out.push_str("%H%M%S"),
            Some('Z') if plain => {
                let mut text = String::new();
                write_offset(&mut text, date.offset(), OffsetStyle::Full);
                literal(&text, &mut out);
            }
            Some('z') if !directive.dot => {
                let style = match (directive.flag, directive.colons) {
                    (Some('#'), 0) | (_, 0) => OffsetStyle::Minutes { colon: false },
                    (_, 1) => OffsetStyle::Minutes { colon: true },
                    (_, 2) => OffsetStyle::Seconds { colon: true },
                    (_, 3) => OffsetStyle::Hours,
                    _ => {
                        // Not a chrono directive; let jiff report it.
                        out.push_str(directive.text);
                        continue;
                    }
                };
                let mut text = String::new();
                write_offset(&mut text, date.offset(), style);
                literal(&text, &mut out);
            }
            Some('+') if plain => literal(&date.to_rfc3339(), &mut out),
            Some('f') if directive.colons == 0 => {
                let nanos = parts.nanosecond;
                let seconds = match directive.width {
                    None if directive.dot => Some(SecondsFormat::AutoSi),
                    None => Some(SecondsFormat::Nanos),
                    Some(3) => Some(SecondsFormat::Millis),
                    Some(6) => Some(SecondsFormat::Micros),
                    Some(9) => Some(SecondsFormat::Nanos),
                    Some(_) => None,
                };
                match seconds {
                    Some(seconds) => {
                        let point = if directive.dot {
                            locale_match!(locale => LC_NUMERIC::DECIMAL_POINT)
                        } else {
                            ""
                        };
                        let mut text = String::new();
                        write_fraction(&mut text, nanos, seconds, point);
                        literal(&text, &mut out);
                    }
                    // Other widths are jiff's own precision setting.
                    None => out.push_str(directive.text),
                }
            }
            _ => out.push_str(directive.text),
        }
    }
    out.push_str(rest);
    out
}

/// A year as chrono wrote it: at least 4 digits, with `-` before negative years and `+` before
/// years past 9999.
fn four_digit_year(year: i32) -> String {
    let mut text = String::new();
    super::write_year(&mut text, year);
    text
}

/// `value` padded the way the directive's flag and width ask: `-` for none, `_` for spaces, and
/// zeros otherwise, to the width or `default_width`.
fn pad(value: i32, directive: &Directive<'_>, default_width: usize) -> String {
    let width = directive
        .width
        .map_or(default_width, |width| width.min(20) as usize);
    let digits = value.unsigned_abs().to_string();
    let sign = if value < 0 { "-" } else { "" };
    match directive.flag {
        Some('-') => format!("{sign}{digits}"),
        Some('_') => format!("{:>width$}", format!("{sign}{digits}")),
        _ => format!(
            "{sign}{digits:0>width$}",
            width = width.saturating_sub(sign.len())
        ),
    }
}

/// At least 2 digits, with `-` before negative values.
fn two_digits(value: i32) -> String {
    if value < 0 {
        format!("-{:02}", value.unsigned_abs())
    } else {
        format!("{value:02}")
    }
}

/// The locale's am or pm text, which is empty for locales without a 12-hour clock.
fn am_pm(locale: Locale, pm: bool) -> &'static str {
    let names = locale_match!(locale => LC_TIME::AM_PM);
    names.get(usize::from(pm)).copied().unwrap_or_default()
}
