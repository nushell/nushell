//! The phrases `date from-human` read with human-date-parser, read the same way.
//!
//! This is a port of human-date-parser 0.3.1 (its `date_time.pest` grammar and the date
//! arithmetic in `lib.rs`; MIT licensed). It differs where human-date-parser misread a phrase.
//! The whole phrase must match, trying every way it can, so `next month` is a month away rather
//! than `next mon(day)` and `this week sunday` is that Sunday. `this hour` and the other units
//! under a day, which panicked, name the current time give or take one unit.

use jiff::{ZonedDateTime, tz::Offset};
use nu_protocol::DateTime;

/// A phrase the grammar matched.
pub(super) struct Phrase(Human);

/// Phrases longer than this aren't tried, so a long string can't make the search slow.
const MAX_LEN: usize = 200;

impl Phrase {
    /// Reads all of `text`, ignoring case and trailing spaces.
    pub(super) fn parse(text: &str) -> Option<Self> {
        (text.len() <= MAX_LEN).then(|| whole(&text.to_lowercase()))?
    }

    /// Reads the longest start of `text` the grammar matches, as human-date-parser did when
    /// other text followed a phrase (`today blah`).
    pub(super) fn parse_prefix(text: &str) -> Option<Self> {
        let end = (0..=MAX_LEN.min(text.len()))
            .rev()
            .find(|&i| text.is_char_boundary(i))?;
        let text = text[..end].to_lowercase();
        let ends: Vec<usize> = text.char_indices().map(|(i, _)| i).skip(1).collect();
        ends.into_iter().rev().find_map(|end| whole(&text[..end]))
    }

    /// The date the phrase names relative to `now`, with the time of day `now` has when it
    /// names only a date, at `now`'s UTC offset. `None` for a date that doesn't exist, such as
    /// `2022-02-30`.
    pub(super) fn date(&self, now: &ZonedDateTime) -> Option<DateTime> {
        let offset = now.offset();
        let now = DateTime::from_civil(now.datetime(), Offset::UTC)?;
        let at = |wall: DateTime, time: DateTime| {
            let (d, t) = (wall.parts(), time.parts());
            DateTime::from_parts(
                d.year,
                d.month,
                d.day,
                t.hour,
                t.minute,
                t.second,
                t.nanosecond,
                offset,
            )
        };
        match self.0.read(now)? {
            Reading::DateTime(wall) => at(wall, wall),
            Reading::Date(day) => at(day, now),
            Reading::Time(time) => at(now, time),
        }
    }
}

/// The first reading of all of the lowercase `text`.
fn whole(text: &str) -> Option<Phrase> {
    human_time(text)
        .into_iter()
        .find(|(_, rest)| rest.trim_start_matches(' ').is_empty())
        .map(|(human, _)| Phrase(human))
}

/// What a phrase names. Each is a wall-clock reading held at UTC.
enum Reading {
    DateTime(DateTime),
    /// A day, at midnight.
    Date(DateTime),
    /// A time of day, on 1970-01-01.
    Time(DateTime),
}

#[derive(Clone, Copy)]
enum Unit {
    Year,
    Month,
    Week,
    Day,
    Hour,
    Minute,
    Second,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Relative {
    This,
    Next,
    Last,
}

/// A weekday, as days since Monday.
type Weekday = u8;

#[derive(Clone, Copy)]
enum Date {
    Today,
    Tomorrow,
    Overmorrow,
    Yesterday,
    Iso(u32, u32, u32),
    DayMonthYear(u32, u8, u32),
    DayMonth(u32, u8),
    RelativeWeekday(Relative, Weekday),
    RelativeWeekWeekday(Relative, Weekday),
    RelativeUnit(Relative, Unit),
    Upcoming(Weekday),
}

#[derive(Clone, Copy)]
struct Time(u32, u32, u32);

type Duration = Vec<(u32, Unit)>;

#[derive(Clone)]
enum Human {
    DateTime(Date, Time),
    Date(Date),
    Time(Time),
    In(Duration),
    Ago(Duration, Option<Box<Human>>),
    Now,
}

impl Human {
    fn read(&self, now: DateTime) -> Option<Reading> {
        Some(match self {
            Self::DateTime(date, time) => {
                let (day, time) = (date.day(now)?, time.read()?);
                Reading::DateTime(day.checked_add_nanos(time_of_day(time))?)
            }
            Self::Date(Date::RelativeUnit(
                relative,
                unit @ (Unit::Hour | Unit::Minute | Unit::Second),
            )) => Reading::DateTime(relative_unit(*relative, *unit, now)?),
            Self::Date(date) => Reading::Date(date.day(now)?),
            Self::Time(time) => Reading::Time(time.read()?),
            Self::In(duration) => Reading::DateTime(apply(duration, now, 1)?),
            Self::Ago(duration, None) => Reading::DateTime(apply(duration, now, -1)?),
            Self::Ago(duration, Some(from)) => {
                let from = match from.read(now)? {
                    Reading::DateTime(wall) => wall,
                    Reading::Date(day) => day.checked_add_nanos(time_of_day(now))?,
                    Reading::Time(time) => midnight(now)?.checked_add_nanos(time_of_day(time))?,
                };
                Reading::DateTime(apply(duration, from, -1)?)
            }
            Self::Now => Reading::DateTime(now),
        })
    }
}

impl Date {
    /// The day, at midnight.
    fn day(&self, now: DateTime) -> Option<DateTime> {
        let today = midnight(now)?;
        match *self {
            Self::Today => Some(today),
            Self::Tomorrow => add_days(today, 1),
            Self::Overmorrow => add_days(today, 2),
            Self::Yesterday => add_days(today, -1),
            Self::Iso(year, month, day) => ymd(year as i32, month, day),
            Self::DayMonthYear(day, month, year) => ymd(year as i32, u32::from(month), day),
            Self::DayMonth(day, month) => ymd(now.year(), u32::from(month), day),
            Self::RelativeWeekday(relative, weekday) => find_weekday(relative, weekday, today),
            Self::RelativeWeekWeekday(relative, weekday) => {
                let week = match relative {
                    Relative::This => 0,
                    Relative::Next => 7,
                    Relative::Last => -7,
                };
                let monday = add_days(today, week - i64::from(weekday_of(today)))?;
                find_weekday(Relative::This, weekday, monday)
            }
            Self::RelativeUnit(relative, unit) => midnight(relative_unit(relative, unit, now)?),
            Self::Upcoming(weekday) => find_weekday(Relative::Next, weekday, today),
        }
    }
}

impl Time {
    /// The time of day, on 1970-01-01.
    fn read(self) -> Option<DateTime> {
        let Self(hour, minute, second) = self;
        if hour >= 24 || minute >= 60 || second >= 60 {
            return None;
        }
        DateTime::from_parts(
            1970,
            1,
            1,
            hour as i8,
            minute as i8,
            second as i8,
            0,
            Offset::UTC,
        )
    }
}

fn midnight(date: DateTime) -> Option<DateTime> {
    let p = date.parts();
    DateTime::from_parts(p.year, p.month, p.day, 0, 0, 0, 0, Offset::UTC)
}

/// Nanoseconds since midnight.
fn time_of_day(date: DateTime) -> i64 {
    midnight(date).map_or(0, |day| date.nanos_since(day) as i64)
}

fn ymd(year: i32, month: u32, day: u32) -> Option<DateTime> {
    let (month, day) = (i8::try_from(month).ok()?, i8::try_from(day).ok()?);
    DateTime::from_parts(year, month, day, 0, 0, 0, 0, Offset::UTC)
}

fn add_days(date: DateTime, days: i64) -> Option<DateTime> {
    date.checked_add_seconds(days.checked_mul(86_400)?)
}

fn weekday_of(date: DateTime) -> Weekday {
    // `parts().weekday` counts from Sunday.
    ((date.parts().weekday + 6) % 7) as Weekday
}

/// The `weekday` on or after `day` (`this`), after it (`next`), or before it (`last`).
fn find_weekday(relative: Relative, weekday: Weekday, day: DateTime) -> Option<DateTime> {
    let current = i64::from(weekday_of(day));
    let target = i64::from(weekday);
    match relative {
        Relative::This if current == target => Some(day),
        Relative::This | Relative::Next => {
            let offset = if target > current {
                target - current
            } else {
                7 - current + target
            };
            add_days(day, offset)
        }
        Relative::Last => {
            let offset = if target >= current {
                7 + current - target
            } else {
                current - target
            };
            add_days(day, -offset)
        }
    }
}

/// `now` (`this`), or one `unit` after (`next`) or before (`last`) it.
fn relative_unit(relative: Relative, unit: Unit, now: DateTime) -> Option<DateTime> {
    match relative {
        Relative::This => Some(now),
        Relative::Next => apply(&[(1, unit)], now, 1),
        Relative::Last => apply(&[(1, unit)], now, -1),
    }
}

/// `date` moved by each amount in turn, forward for a `sign` of 1 and back for -1. A year
/// that has no such day (February 29) names no date; a month past the end of the next month's
/// days ends on its last day.
fn apply(duration: &[(u32, Unit)], mut date: DateTime, sign: i64) -> Option<DateTime> {
    for &(count, unit) in duration {
        let count = sign * i64::from(count);
        let p = date.parts();
        date = match unit {
            Unit::Year => DateTime::from_parts(
                p.year.checked_add(i32::try_from(count).ok()?)?,
                p.month,
                p.day,
                p.hour,
                p.minute,
                p.second,
                p.nanosecond,
                Offset::UTC,
            )?,
            Unit::Month => {
                let months = i64::from(p.year) * 12 + i64::from(p.month) - 1 + count;
                let year = i32::try_from(months.div_euclid(12)).ok()?;
                let month = (months.rem_euclid(12) + 1) as i8;
                let last = (28..=31)
                    .rev()
                    .find(|&day| ymd(year, month as u32, day).is_some())?;
                DateTime::from_parts(
                    year,
                    month,
                    p.day.min(last as i8),
                    p.hour,
                    p.minute,
                    p.second,
                    p.nanosecond,
                    Offset::UTC,
                )?
            }
            Unit::Week => add_days(date, count.checked_mul(7)?)?,
            Unit::Day => add_days(date, count)?,
            Unit::Hour => date.checked_add_seconds(count * 3600)?,
            Unit::Minute => date.checked_add_seconds(count * 60)?,
            Unit::Second => date.checked_add_seconds(count)?,
        };
    }
    Some(date)
}

/// Every way a rule matches the start of the text, in the grammar's order, with the text left.
type Parses<'a, T> = Vec<(T, &'a str)>;

/// The whitespace the grammar skips between the parts of a rule: spaces only.
fn spaces(s: &str) -> &str {
    s.trim_start_matches(' ')
}

fn num(s: &str) -> Option<(u32, &str)> {
    let len = s.bytes().take_while(u8::is_ascii_digit).count();
    if len == 0 {
        return None;
    }
    Some((s[..len].parse().ok()?, &s[len..]))
}

/// One of `names` (long form, then short), as its index.
fn name<'a>(s: &'a str, names: &[(&str, &str)]) -> Parses<'a, u8> {
    let mut out = Vec::new();
    for (i, (long, short)) in names.iter().enumerate() {
        let forms: &[&str] = if long == short {
            &[long]
        } else {
            &[long, short]
        };
        for form in forms {
            if let Some(rest) = s.strip_prefix(form) {
                out.push((i as u8, rest));
            }
        }
    }
    out
}

const WEEKDAYS: [(&str, &str); 7] = [
    ("monday", "mon"),
    ("tuesday", "tue"),
    ("wednesday", "wed"),
    ("thursday", "thu"),
    ("friday", "fri"),
    ("saturday", "sat"),
    ("sunday", "sun"),
];

const MONTHS: [(&str, &str); 12] = [
    ("january", "jan"),
    ("february", "feb"),
    ("march", "mar"),
    ("april", "apr"),
    ("may", "may"),
    ("june", "jun"),
    ("july", "jul"),
    ("august", "aug"),
    ("september", "sep"),
    ("october", "oct"),
    ("november", "nov"),
    ("december", "dec"),
];

fn weekday(s: &str) -> Parses<'_, Weekday> {
    name(s, &WEEKDAYS)
}

/// A month, 1 to 12.
fn month(s: &str) -> Parses<'_, u8> {
    name(s, &MONTHS)
        .into_iter()
        .map(|(i, rest)| (i + 1, rest))
        .collect()
}

/// `word` with an optional `s` after it (spaces allowed between), with the `s` first.
fn plural<'a>(s: &'a str, word: &str) -> Vec<&'a str> {
    let Some(rest) = s.strip_prefix(word) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if let Some(after) = spaces(rest).strip_prefix('s') {
        out.push(after);
    }
    out.push(rest);
    out
}

fn unit(s: &str) -> Parses<'_, Unit> {
    let letter = |c: char| s.strip_prefix(c).into_iter().collect::<Vec<_>>();
    [
        (Unit::Year, plural(s, "year")),
        (Unit::Month, plural(s, "month")),
        (Unit::Week, plural(s, "week")),
        (Unit::Day, plural(s, "day")),
        (Unit::Day, letter('d')),
        (Unit::Hour, plural(s, "hour")),
        (Unit::Hour, letter('h')),
        (Unit::Minute, plural(s, "minute")),
        (Unit::Minute, plural(s, "min")),
        (Unit::Minute, letter('m')),
        (Unit::Second, plural(s, "second")),
        (Unit::Second, plural(s, "sec")),
        (Unit::Second, letter('s')),
    ]
    .into_iter()
    .flat_map(|(unit, rests)| rests.into_iter().map(move |rest| (unit, rest)))
    .collect()
}

fn relative(s: &str) -> Option<(Relative, &str)> {
    [
        ("this", Relative::This),
        ("next", Relative::Next),
        ("last", Relative::Last),
    ]
    .into_iter()
    .find_map(|(word, relative)| s.strip_prefix(word).map(|rest| (relative, rest)))
}

fn date(s: &str) -> Parses<'_, Date> {
    let mut out = Vec::new();
    for (word, date) in [
        ("today", Date::Today),
        ("tomorrow", Date::Tomorrow),
        ("overmorrow", Date::Overmorrow),
        ("yesterday", Date::Yesterday),
    ] {
        if let Some(rest) = s.strip_prefix(word) {
            out.push((date, rest));
        }
    }
    // `YYYY-MM-DD`, with no spaces inside.
    if let Some((year, rest)) = num(s)
        && let Some((month, rest)) = rest.strip_prefix('-').and_then(num)
        && let Some((day, rest)) = rest.strip_prefix('-').and_then(num)
    {
        out.push((Date::Iso(year, month, day), rest));
    }
    if let Some((day, rest)) = num(s) {
        for (month, rest) in month(spaces(rest)) {
            if let Some((year, rest)) = num(spaces(rest)) {
                out.push((Date::DayMonthYear(day, month, year), rest));
            }
        }
        for (month, rest) in month(spaces(rest)) {
            out.push((Date::DayMonth(day, month), rest));
        }
    }
    if let Some((relative, rest)) = relative(s) {
        let rest = spaces(rest);
        for (day, rest) in weekday(rest) {
            out.push((Date::RelativeWeekday(relative, day), rest));
        }
        for rest in plural(rest, "week") {
            for (day, rest) in weekday(spaces(rest)) {
                out.push((Date::RelativeWeekWeekday(relative, day), rest));
            }
        }
        for (unit, rest) in unit(rest) {
            out.push((Date::RelativeUnit(relative, unit), rest));
        }
    }
    if let Some(rest) = s.strip_prefix("upcoming") {
        for (day, rest) in weekday(spaces(rest)) {
            out.push((Date::Upcoming(day), rest));
        }
    }
    for (day, rest) in weekday(s) {
        out.push((Date::Upcoming(day), rest));
    }
    out
}

/// `H:M` or `H:M:S`, with no spaces inside.
fn time(s: &str) -> Parses<'_, Time> {
    let mut out = Vec::new();
    if let Some((hour, rest)) = num(s)
        && let Some((minute, rest)) = rest.strip_prefix(':').and_then(num)
    {
        if let Some((second, rest)) = rest.strip_prefix(':').and_then(num) {
            out.push((Time(hour, minute, second), rest));
        }
        out.push((Time(hour, minute, 0), rest));
    }
    out
}

/// `at` and the spaces after it, if there.
fn optional_word<'a>(s: &'a str, word: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    if let Some(rest) = s.strip_prefix(word) {
        out.push(spaces(rest));
    }
    out.push(s);
    out
}

fn duration(s: &str) -> Parses<'_, Duration> {
    fn quantifier(s: &str) -> Parses<'_, (u32, Unit)> {
        let Some((count, rest)) = num(s) else {
            return Vec::new();
        };
        unit(spaces(rest))
            .into_iter()
            .map(|(unit, rest)| ((count, unit), rest))
            .collect()
    }
    /// The quantifiers after `so_far`, longest first, then `so_far` itself.
    fn more<'a>(so_far: Duration, s: &'a str, out: &mut Parses<'a, Duration>) {
        let after = spaces(s);
        let mut separators = vec![after];
        for separator in [",", "and"] {
            if let Some(rest) = after.strip_prefix(separator) {
                separators.insert(0, spaces(rest));
            }
        }
        for start in separators {
            for (next, rest) in quantifier(start) {
                let mut longer = so_far.clone();
                longer.push(next);
                more(longer, rest, out);
            }
        }
        out.push((so_far, s));
    }
    let mut out = Vec::new();
    for (first, rest) in quantifier(s) {
        more(vec![first], rest, &mut out);
    }
    for article in ["an", "a"] {
        if let Some(rest) = s.strip_prefix(article) {
            for (unit, rest) in unit(spaces(rest)) {
                out.push((vec![(1, unit)], rest));
            }
        }
    }
    out
}

fn human_time(s: &str) -> Parses<'_, Human> {
    let mut out = Vec::new();
    for (day, rest) in date(s) {
        for rest in optional_word(spaces(rest), "at") {
            for (time, rest) in time(rest) {
                out.push((Human::DateTime(day, time), rest));
            }
        }
    }
    for (time, rest) in time(s) {
        for rest in optional_word(spaces(rest), ",") {
            for (day, rest) in date(rest) {
                out.push((Human::DateTime(day, time), rest));
            }
        }
    }
    out.extend(
        date(s)
            .into_iter()
            .map(|(day, rest)| (Human::Date(day), rest)),
    );
    for start in optional_word(s, "at") {
        out.extend(
            time(start)
                .into_iter()
                .map(|(time, rest)| (Human::Time(time), rest)),
        );
    }
    if let Some(rest) = s.strip_prefix("in") {
        out.extend(
            duration(spaces(rest))
                .into_iter()
                .map(|(duration, rest)| (Human::In(duration), rest)),
        );
    }
    for (duration, rest) in duration(s) {
        let Some(rest) = spaces(rest).strip_prefix("ago") else {
            continue;
        };
        for start in optional_word(spaces(rest), "at") {
            for (from, rest) in human_time(start) {
                out.push((Human::Ago(duration.clone(), Some(Box::new(from))), rest));
            }
        }
        out.push((Human::Ago(duration, None), rest));
    }
    if let Some(rest) = s.strip_prefix("now") {
        out.push((Human::Now, rest));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use pure_rust_locales::Locale;

    /// human-date-parser 0.3.1's own cases (`src/tests.rs`), then the phrases it misread.
    const CASES: &[(&str, &str)] = &[
        ("15:10", "2010-01-01 15:10:00"),
        ("Today 18:30", "2010-01-01 18:30:00"),
        ("Yesterday 18:30", "2009-12-31 18:30:00"),
        ("Tomorrow 18:30", "2010-01-02 18:30:00"),
        ("Overmorrow 18:30", "2010-01-03 18:30:00"),
        ("2022-11-07 13:25:30", "2022-11-07 13:25:30"),
        ("07 February 2015", "2015-02-07 00:00:00"),
        ("07 February", "2010-02-07 00:00:00"),
        ("15:20 Friday", "2010-01-08 15:20:00"),
        ("This Friday 17:00", "2010-01-01 17:00:00"),
        ("Next Friday 17:00", "2010-01-08 17:00:00"),
        ("13:25, Next Tuesday", "2010-01-05 13:25:00"),
        ("Last Friday at 19:45", "2009-12-25 19:45:00"),
        ("Next week", "2010-01-08 00:00:00"),
        ("This week", "2010-01-01 00:00:00"),
        ("Last week", "2009-12-25 00:00:00"),
        ("Next week Monday", "2010-01-04 00:00:00"),
        ("This week Friday", "2010-01-01 00:00:00"),
        ("This week Monday", "2009-12-28 00:00:00"),
        ("Last week Tuesday", "2009-12-22 00:00:00"),
        ("Last Monday", "2009-12-28 00:00:00"),
        ("Last Tueday", "2009-12-29 00:00:00"),
        ("Last Wednesday", "2009-12-30 00:00:00"),
        ("Last Thursday", "2009-12-31 00:00:00"),
        ("Last Friday", "2009-12-25 00:00:00"),
        ("Last Saturday", "2009-12-26 00:00:00"),
        ("Last Sunday", "2009-12-27 00:00:00"),
        ("This Monday", "2010-01-04 00:00:00"),
        ("This Tueday", "2010-01-05 00:00:00"),
        ("This Wednesday", "2010-01-06 00:00:00"),
        ("This Thursday", "2010-01-07 00:00:00"),
        ("This Friday", "2010-01-01 00:00:00"),
        ("This Saturday", "2010-01-02 00:00:00"),
        ("This Sunday", "2010-01-03 00:00:00"),
        ("Next Monday", "2010-01-04 00:00:00"),
        ("Next Tueday", "2010-01-05 00:00:00"),
        ("Next Wednesday", "2010-01-06 00:00:00"),
        ("Next Thursday", "2010-01-07 00:00:00"),
        ("Next Friday", "2010-01-08 00:00:00"),
        ("Next Saturday", "2010-01-02 00:00:00"),
        ("Next Sunday", "2010-01-03 00:00:00"),
        ("In 3 days", "2010-01-04 00:00:00"),
        ("In 2 hours", "2010-01-01 02:00:00"),
        ("In 5 minutes and 30 seconds", "2010-01-01 00:05:30"),
        ("10 seconds ago", "2009-12-31 23:59:50"),
        ("10 hours and 5 minutes ago", "2009-12-31 13:55:00"),
        (
            "2 hours, 32 minutes and 7 seconds ago",
            "2009-12-31 21:27:53",
        ),
        (
            "1 years, 2 months, 3 weeks, 5 days, 8 hours, 17 minutes and 45 seconds ago",
            "2008-10-05 15:42:15",
        ),
        (
            "1 year, 1 month, 1 week, 1 day, 1 hour, 1 minute and 1 second ago",
            "2008-11-22 22:58:59",
        ),
        ("A year ago", "2009-01-01 00:00:00"),
        ("A month ago", "2009-12-01 00:00:00"),
        ("3 months ago", "2009-10-01 00:00:00"),
        ("6 months ago", "2009-07-01 00:00:00"),
        ("7 months ago", "2009-06-01 00:00:00"),
        ("In 7 months", "2010-08-01 00:00:00"),
        ("A week ago", "2009-12-25 00:00:00"),
        ("A day ago", "2009-12-31 00:00:00"),
        ("An hour ago", "2009-12-31 23:00:00"),
        ("A minute ago", "2009-12-31 23:59:00"),
        ("A second ago", "2009-12-31 23:59:59"),
        ("now", "2010-01-01 00:00:00"),
        ("Overmorrow", "2010-01-03 00:00:00"),
        ("7 days ago at 04:00", "2009-12-25 04:00:00"),
        ("12 hours ago at 04:00", "2009-12-31 16:00:00"),
        ("12 hours ago at today", "2009-12-31 12:00:00"),
        ("12 hours ago at 7 days ago", "2009-12-24 12:00:00"),
        ("7 days ago at 7 days ago", "2009-12-18 00:00:00"),
        // human-date-parser read these as `next mon`, `this weeks` and a panic.
        ("Next month", "2010-02-01 00:00:00"),
        ("This week Sunday", "2010-01-03 00:00:00"),
        ("Next week Saturday", "2010-01-09 00:00:00"),
        ("Next hour", "2010-01-01 01:00:00"),
        ("Last minute", "2009-12-31 23:59:00"),
        ("in 3d", "2010-01-04 00:00:00"),
        ("upcoming friday", "2010-01-08 00:00:00"),
        // Text after a phrase is ignored, as human-date-parser ignored it.
        ("today blah", "2010-01-01 00:00:00"),
        ("next fridayx", "2010-01-08 00:00:00"),
    ];

    #[test]
    fn phrases_read_as_human_date_parser_read_them() {
        // 2010-01-01T00:00:00Z, a Friday.
        let now = jiff::Timestamp::from_second(1_262_304_000)
            .unwrap()
            .to_zoned_datetime(jiff::tz::TimeZone::UTC);
        for (text, expected) in CASES {
            let phrase = Phrase::parse(text)
                .or_else(|| Phrase::parse_prefix(text))
                .unwrap_or_else(|| panic!("{text:?} should parse"));
            let date = phrase
                .date(&now)
                .unwrap_or_else(|| panic!("{text:?} should name a date"));
            assert_eq!(
                &date.format("%F %T", Locale::POSIX).unwrap(),
                expected,
                "{text:?}"
            );
        }
        assert!(Phrase::parse("2023-11-31").unwrap().date(&now).is_none());
        // Text that isn't ASCII, or whose lowercase form is longer, must not panic.
        for text in [
            "mañana",
            "today ü",
            "İstanbul 10:30",
            "in 2 días",
            "ü",
            "10:3ü",
            "next mönday",
        ] {
            let _ = Phrase::parse(text);
            let _ = Phrase::parse_prefix(text);
        }
        assert!(Phrase::parse("tomorrow 10am").is_none());
    }
}
