// This file is a port of the strftime parser and field resolver of chrono 0.4.45
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

//! Reading text with a strftime-style format, as chrono's `parse_from_str` functions did.
//!
//! This is a port of chrono 0.4.45's format parser (`format/strftime.rs`, `format/parse.rs`,
//! `format/scan.rs` and `format/parsed.rs`), so `into datetime --format`, `seq date` and
//! `from kdl` read text as they did when nushell used chrono, except that `%s` takes a sign,
//! `%Z` reads only `UTC`, `GMT` and offsets, and nushell's `%J` and `%Q` read `%Y%m%d` and
//! `%H%M%S`. The format splits into items, the items read the text into the fields of a
//! [`ParsedDate`], and the fields resolve to an instant, a wall-clock date and time, a date or
//! a time of day, as chrono's `DateTime`, `NaiveDateTime`, `NaiveDate` and `NaiveTime` did.

use super::{
    DateParseError, DateTime, MAX_YEAR, MIN_YEAR, NANOS_PER_SECOND, SECONDS_PER_DAY,
    civil_from_days, days_from_civil, days_in_month,
};
use jiff::tz::Offset;

use DateParseError::{BadFormat, Impossible, Invalid, NotEnough, OutOfRange, TooLong, TooShort};

type ParseResult<T> = Result<T, DateParseError>;

/// The fields a format read from text, before they resolve to a date, a time or both.
///
/// A field read twice must have the same value both times, and the fields must agree with
/// each other when they resolve: `%s` with a `%Y` of another year names no date.
#[derive(Clone, Debug, Default)]
pub struct ParsedDate {
    year: Option<i32>,
    year_div_100: Option<i32>,
    year_mod_100: Option<i32>,
    isoyear: Option<i32>,
    isoyear_mod_100: Option<i32>,
    quarter: Option<u32>,
    month: Option<u32>,
    week_from_sun: Option<u32>,
    week_from_mon: Option<u32>,
    isoweek: Option<u32>,
    weekday: Option<Weekday>,
    ordinal: Option<u32>,
    day: Option<u32>,
    hour_div_12: Option<u32>,
    hour_mod_12: Option<u32>,
    minute: Option<u32>,
    second: Option<u32>,
    nanosecond: Option<u32>,
    timestamp: Option<i64>,
    /// `%s` had a minus sign, which `-0` keeps where the number doesn't.
    timestamp_negative: bool,
    offset: Option<i32>,
}

/// A wall-clock time of day, as [`ParsedDate::to_naive_time`] reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeOfDay {
    pub hour: i8,
    pub minute: i8,
    /// 0 to 60. Second 60, which UTC gives a leap second, is the next minute's second 0 once
    /// the time is on a day, so `23:59:60` is midnight of the next day.
    pub second: i8,
    /// 0 to 999,999,999.
    pub nanosecond: i32,
}

impl TimeOfDay {
    /// This time on the given day, as a wall-clock reading held at UTC. Second 60 is the next
    /// minute's second 0.
    pub fn on(self, year: i32, month: i8, day: i8) -> Option<DateTime> {
        DateTime::from_parts(
            year,
            month,
            day,
            self.hour,
            self.minute,
            self.second,
            self.nanosecond,
            Offset::UTC,
        )
    }

    fn seconds_of_day(self) -> i64 {
        i64::from(self.hour) * 3600 + i64::from(self.minute) * 60 + i64::from(self.second)
    }
}

impl ParsedDate {
    /// Reads `text` with `format`. The whole text must match.
    pub fn parse(text: &str, format: &str) -> ParseResult<Self> {
        let mut parsed = Self::default();
        match parse_items(&mut parsed, text, &items(format))? {
            "" => Ok(parsed),
            _ => Err(TooLong),
        }
    }

    /// The instant the fields name, as chrono's `DateTime::<FixedOffset>::parse_from_str` read
    /// it: a date, a time and an offset, or a Unix timestamp (`%s`) at UTC unless an offset is
    /// given.
    pub fn to_datetime(&self) -> ParseResult<DateTime> {
        let offset = match (self.offset, self.timestamp) {
            (Some(offset), _) => offset,
            (None, Some(_)) => 0,
            (None, None) => return Err(NotEnough),
        };
        let (date, time) = self.to_naive_datetime_with_offset(offset)?;
        if !(-86_399..=86_399).contains(&offset) {
            return Err(OutOfRange);
        }
        let offset = Offset::from_seconds(offset).map_err(|_| OutOfRange)?;
        date.at(time, offset).ok_or(Impossible)
    }

    /// The wall-clock date and time the fields name, held as that reading at UTC, as chrono's
    /// `NaiveDateTime::parse_from_str` read it. An offset in the text is not applied.
    pub fn to_naive_datetime(&self) -> ParseResult<DateTime> {
        let (date, time) = self.to_naive_datetime_with_offset(0)?;
        date.at(time, Offset::UTC).ok_or(OutOfRange)
    }

    /// The UTC offset the text gave with `%z` or `%Z`, if any.
    pub fn offset(&self) -> ParseResult<Option<Offset>> {
        match self.offset {
            None => Ok(None),
            Some(seconds) if (-86_399..=86_399).contains(&seconds) => Offset::from_seconds(seconds)
                .map(Some)
                .map_err(|_| OutOfRange),
            Some(_) => Err(OutOfRange),
        }
    }

    /// The fields with minutes of 0 when the text gave the hour but no minutes, so `10` read
    /// with `%H` or `10 AM` with `%I %p` is 10:00, as seconds left out are 0. chrono's readings
    /// needed the minutes. An hour from `%I` without `%p` is still no time of day.
    pub fn with_missing_minutes_zero(mut self) -> Self {
        if self.minute.is_none() && self.hour_div_12.is_some() && self.hour_mod_12.is_some() {
            self.minute = Some(0);
        }
        self
    }

    /// The wall-clock date and time the fields name, or midnight on their date when they have
    /// no time of day, held as that reading at UTC. Only a reading with no time field at all
    /// falls back to the date: a time missing a field (`%I` without `%p`), or a contradicting
    /// or out-of-range field, is an error, where chrono's readings one after another would drop
    /// it.
    pub fn to_wall_clock(&self) -> ParseResult<DateTime> {
        match self.to_naive_datetime() {
            Err(NotEnough) if !self.has_time_of_day() => self.to_naive_date(),
            result => result,
        }
    }

    /// Whether any field of a time of day was read: an hour, AM or PM, minutes, seconds or a
    /// fraction.
    fn has_time_of_day(&self) -> bool {
        self.hour_div_12.is_some()
            || self.hour_mod_12.is_some()
            || self.minute.is_some()
            || self.second.is_some()
            || self.nanosecond.is_some()
    }

    /// Midnight on the date the fields name, held as that reading at UTC, as chrono's
    /// `NaiveDate::parse_from_str` read it. Time fields are not checked.
    pub fn to_naive_date(&self) -> ParseResult<DateTime> {
        let midnight = TimeOfDay {
            hour: 0,
            minute: 0,
            second: 0,
            nanosecond: 0,
        };
        self.resolve_date()?
            .at(midnight, Offset::UTC)
            .ok_or(OutOfRange)
    }

    /// The time of day the fields name, as chrono's `NaiveTime::parse_from_str` read it. Date
    /// fields are not checked.
    pub fn to_naive_time(&self) -> ParseResult<TimeOfDay> {
        let hour_div_12 = match self.hour_div_12 {
            Some(v @ 0..=1) => v,
            Some(_) => return Err(OutOfRange),
            None => return Err(NotEnough),
        };
        let hour_mod_12 = match self.hour_mod_12 {
            Some(v @ 0..=11) => v,
            Some(_) => return Err(OutOfRange),
            None => return Err(NotEnough),
        };
        let minute = match self.minute {
            Some(v @ 0..=59) => v,
            Some(_) => return Err(OutOfRange),
            None => return Err(NotEnough),
        };
        // Seconds and nanoseconds may be left out; second 60 is a leap second, which a day
        // carries to the next minute.
        let second = match self.second.unwrap_or(0) {
            v @ 0..=60 => v,
            _ => return Err(OutOfRange),
        };
        let nanosecond = match self.nanosecond {
            Some(v @ 0..=999_999_999) if self.second.is_some() => v,
            Some(0..=999_999_999) => return Err(NotEnough),
            Some(_) => return Err(OutOfRange),
            None => 0,
        };
        Ok(TimeOfDay {
            hour: (hour_div_12 * 12 + hour_mod_12) as i8,
            minute: minute as i8,
            second: second as i8,
            nanosecond: nanosecond as i32,
        })
    }

    /// The date the fields name, checked against every date field given.
    fn resolve_date(&self) -> ParseResult<Date> {
        let given_year = resolve_year(self.year, self.year_div_100, self.year_mod_100)?;
        // No directive reads an ISO century.
        let given_isoyear = resolve_year(self.isoyear, None, self.isoyear_mod_100)?;

        let verify_ymd = |date: Date| {
            let (year, month, day) = date.year_month_day();
            let (year_div_100, year_mod_100) = if year >= 0 {
                (Some(year / 100), Some(year % 100))
            } else {
                (None, None)
            };
            self.year.unwrap_or(year) == year
                && self.year_div_100.or(year_div_100) == year_div_100
                && self.year_mod_100.or(year_mod_100) == year_mod_100
                && self.month.unwrap_or(month) == month
                && self.day.unwrap_or(day) == day
        };
        let verify_isoweekdate = |date: Date| {
            let (isoyear, isoweek) = date.iso_week();
            let weekday = date.weekday();
            let isoyear_mod_100 = (isoyear >= 0).then_some(isoyear % 100);
            self.isoyear.unwrap_or(isoyear) == isoyear
                && self.isoyear_mod_100.or(isoyear_mod_100) == isoyear_mod_100
                && self.isoweek.unwrap_or(isoweek) == isoweek
                && self.weekday.unwrap_or(weekday) == weekday
        };
        let verify_ordinal = |date: Date| {
            let ordinal = date.ordinal();
            let week_from_sun = date.weeks_from(Weekday::SUNDAY);
            let week_from_mon = date.weeks_from(Weekday::MONDAY);
            self.ordinal.unwrap_or(ordinal) == ordinal
                && self.week_from_sun.map_or(week_from_sun, |v| v as i32) == week_from_sun
                && self.week_from_mon.map_or(week_from_mon, |v| v as i32) == week_from_mon
        };

        // Build a date from the first complete set of fields, then check it against the rest.
        let (verified, date) = match (given_year, given_isoyear, self) {
            (
                Some(year),
                _,
                &Self {
                    month: Some(month),
                    day: Some(day),
                    ..
                },
            ) => {
                let date = Date::from_ymd(year, month, day).ok_or(OutOfRange)?;
                (verify_isoweekdate(date) && verify_ordinal(date), date)
            }
            (
                Some(year),
                _,
                &Self {
                    ordinal: Some(ordinal),
                    ..
                },
            ) => {
                let date = Date::from_yo(year, ordinal).ok_or(OutOfRange)?;
                let verified = verify_ymd(date) && verify_isoweekdate(date) && verify_ordinal(date);
                (verified, date)
            }
            (
                Some(year),
                _,
                &Self {
                    week_from_sun: Some(week),
                    weekday: Some(weekday),
                    ..
                },
            ) => {
                let date = resolve_week_date(year, week, weekday, Weekday::SUNDAY)?;
                let verified = verify_ymd(date) && verify_isoweekdate(date) && verify_ordinal(date);
                (verified, date)
            }
            (
                Some(year),
                _,
                &Self {
                    week_from_mon: Some(week),
                    weekday: Some(weekday),
                    ..
                },
            ) => {
                let date = resolve_week_date(year, week, weekday, Weekday::MONDAY)?;
                let verified = verify_ymd(date) && verify_isoweekdate(date) && verify_ordinal(date);
                (verified, date)
            }
            (
                _,
                Some(isoyear),
                &Self {
                    isoweek: Some(isoweek),
                    weekday: Some(weekday),
                    ..
                },
            ) => {
                let date = Date::from_isoywd(isoyear, isoweek, weekday).ok_or(OutOfRange)?;
                (verify_ymd(date) && verify_ordinal(date), date)
            }
            _ => return Err(NotEnough),
        };

        if !verified {
            return Err(Impossible);
        }
        if let Some(quarter) = self.quarter
            && quarter != (date.year_month_day().1 - 1) / 3 + 1
        {
            return Err(Impossible);
        }
        Ok(date)
    }

    /// The wall-clock date and time, from the date and time fields or else from the Unix
    /// timestamp read at `offset` seconds east of UTC. A timestamp given with the fields must
    /// agree with them.
    fn to_naive_datetime_with_offset(&self, offset: i32) -> ParseResult<(Date, TimeOfDay)> {
        let date = self.resolve_date();
        let time = self.to_naive_time();
        if let (Ok(date), Ok(time)) = (date, time) {
            let timestamp = date.days * SECONDS_PER_DAY + time.seconds_of_day() - i64::from(offset);
            if let Some((given, _)) = self.signed_timestamp() {
                // chrono took either timestamp for a leap second: its own, second 59's, or the
                // next minute's, which second 60 is here.
                let leap = time.second == 60;
                if given != timestamp && !(leap && given == timestamp - 1) {
                    return Err(Impossible);
                }
            }
            return Ok((date, time));
        }
        let Some((timestamp, nanosecond)) = self.signed_timestamp() else {
            date?;
            return Err(time.err().unwrap_or(NotEnough));
        };
        match (date, time) {
            (Err(OutOfRange), _) | (_, Err(OutOfRange)) => return Err(OutOfRange),
            (Err(Impossible), _) | (_, Err(Impossible)) => return Err(Impossible),
            _ => {}
        }

        // Fill the year, day of the year, hour, minute and second from the timestamp, then
        // resolve with every field so the ones given are checked against it.
        let mut local = timestamp.checked_add(i64::from(offset)).ok_or(OutOfRange)?;
        if !Date::in_range(local.div_euclid(SECONDS_PER_DAY)) {
            return Err(OutOfRange);
        }
        let mut parsed = self.clone();
        parsed.nanosecond = nanosecond;
        if parsed.second == Some(60) {
            // Second 60 follows second 59 of the timestamp's minute, or the timestamp is the
            // next minute's second 0, which second 60 is.
            match local.rem_euclid(60) {
                59 => {}
                0 => local -= 1,
                _ => return Err(Impossible),
            }
        } else {
            parsed.set_second(local.rem_euclid(60))?;
        }
        let date = Date {
            days: local.div_euclid(SECONDS_PER_DAY),
        };
        let of_day = local.rem_euclid(SECONDS_PER_DAY);
        parsed.set_year(i64::from(date.year_month_day().0))?;
        parsed.set_ordinal(i64::from(date.ordinal()))?;
        parsed.set_hour(of_day / 3600)?;
        parsed.set_minute(of_day / 60 % 60)?;
        Ok((parsed.resolve_date()?, parsed.to_naive_time()?))
    }

    /// The Unix timestamp in whole seconds, rounded down, and the nanoseconds after it. chrono
    /// added a fraction forward from the timestamp; one after a negative timestamp counts back
    /// from it here, so `-1.5` is a second and a half before 1970.
    fn signed_timestamp(&self) -> Option<(i64, Option<u32>)> {
        let timestamp = self.timestamp?;
        Some(match self.nanosecond {
            Some(nanos) if (timestamp < 0 || self.timestamp_negative) && nanos > 0 => {
                (timestamp - 1, Some(NANOS_PER_SECOND - nanos))
            }
            nanosecond => (timestamp, nanosecond),
        })
    }

    fn set_numeric(&mut self, numeric: Numeric, value: i64) -> ParseResult<()> {
        match numeric {
            Numeric::Year => set_if_consistent(
                &mut self.year,
                i32::try_from(value).map_err(|_| OutOfRange)?,
            ),
            Numeric::YearDiv100 => set_century(&mut self.year_div_100, value),
            Numeric::YearMod100 => set_two_digits(&mut self.year_mod_100, value),
            Numeric::IsoYear => set_if_consistent(
                &mut self.isoyear,
                i32::try_from(value).map_err(|_| OutOfRange)?,
            ),
            Numeric::IsoYearMod100 => set_two_digits(&mut self.isoyear_mod_100, value),
            Numeric::Quarter => set_in_range(&mut self.quarter, value, 1..=4),
            Numeric::Month => set_in_range(&mut self.month, value, 1..=12),
            Numeric::Day => set_in_range(&mut self.day, value, 1..=31),
            Numeric::WeekFromSun => set_in_range(&mut self.week_from_sun, value, 0..=53),
            Numeric::WeekFromMon => set_in_range(&mut self.week_from_mon, value, 0..=53),
            Numeric::IsoWeek => set_in_range(&mut self.isoweek, value, 1..=53),
            Numeric::NumDaysFromSun => match value {
                0 => self.set_weekday(Weekday::SUNDAY),
                1..=6 => self.set_weekday(Weekday(value as u8 - 1)),
                _ => Err(OutOfRange),
            },
            Numeric::WeekdayFromMon => match value {
                1..=7 => self.set_weekday(Weekday(value as u8 - 1)),
                _ => Err(OutOfRange),
            },
            Numeric::Ordinal => self.set_ordinal(value),
            Numeric::Hour => self.set_hour(value),
            Numeric::Hour12 => {
                if !(1..=12).contains(&value) {
                    return Err(OutOfRange);
                }
                set_if_consistent(&mut self.hour_mod_12, (value % 12) as u32)
            }
            Numeric::Minute => self.set_minute(value),
            Numeric::Second => self.set_second(value),
            Numeric::Nanosecond => self.set_nanosecond(value),
            Numeric::Timestamp => set_if_consistent(&mut self.timestamp, value),
        }
    }

    fn set_year(&mut self, value: i64) -> ParseResult<()> {
        self.set_numeric(Numeric::Year, value)
    }

    fn set_month(&mut self, value: i64) -> ParseResult<()> {
        set_in_range(&mut self.month, value, 1..=12)
    }

    fn set_weekday(&mut self, value: Weekday) -> ParseResult<()> {
        set_if_consistent(&mut self.weekday, value)
    }

    fn set_ordinal(&mut self, value: i64) -> ParseResult<()> {
        set_in_range(&mut self.ordinal, value, 1..=366)
    }

    fn set_hour(&mut self, value: i64) -> ParseResult<()> {
        if !(0..=23).contains(&value) {
            return Err(OutOfRange);
        }
        set_if_consistent(&mut self.hour_div_12, (value / 12) as u32)?;
        set_if_consistent(&mut self.hour_mod_12, (value % 12) as u32)
    }

    fn set_minute(&mut self, value: i64) -> ParseResult<()> {
        set_in_range(&mut self.minute, value, 0..=59)
    }

    fn set_second(&mut self, value: i64) -> ParseResult<()> {
        set_in_range(&mut self.second, value, 0..=60)
    }

    fn set_nanosecond(&mut self, value: i64) -> ParseResult<()> {
        set_in_range(&mut self.nanosecond, value, 0..=999_999_999)
    }

    fn set_offset(&mut self, value: i32) -> ParseResult<()> {
        set_if_consistent(&mut self.offset, value)
    }
}

/// Sets a field read for the first time; a field read again must have the same value.
fn set_if_consistent<T: PartialEq>(field: &mut Option<T>, value: T) -> ParseResult<()> {
    match field {
        Some(old) if *old != value => Err(Impossible),
        _ => {
            *field = Some(value);
            Ok(())
        }
    }
}

fn set_in_range(
    field: &mut Option<u32>,
    value: i64,
    range: std::ops::RangeInclusive<i64>,
) -> ParseResult<()> {
    if !range.contains(&value) {
        return Err(OutOfRange);
    }
    set_if_consistent(field, value as u32)
}

fn set_century(field: &mut Option<i32>, value: i64) -> ParseResult<()> {
    let value = i32::try_from(value).map_err(|_| OutOfRange)?;
    if value < 0 {
        return Err(OutOfRange);
    }
    set_if_consistent(field, value)
}

fn set_two_digits(field: &mut Option<i32>, value: i64) -> ParseResult<()> {
    if !(0..100).contains(&value) {
        return Err(OutOfRange);
    }
    set_if_consistent(field, value as i32)
}

/// The full year from a full year (`%Y`), a century (`%C`) and a year of the century (`%y`),
/// checking that the ones given agree. A year of the century alone below 70 is in the 2000s.
fn resolve_year(y: Option<i32>, q: Option<i32>, r: Option<i32>) -> ParseResult<Option<i32>> {
    match (y, q, r) {
        (y, None, None) => Ok(y),
        (Some(y), q, r @ (Some(0..=99) | None)) => {
            if y < 0 {
                return Err(Impossible);
            }
            if q.unwrap_or(y / 100) == y / 100 && r.unwrap_or(y % 100) == y % 100 {
                Ok(Some(y))
            } else {
                Err(Impossible)
            }
        }
        (None, Some(q), Some(r @ 0..=99)) => {
            if q < 0 {
                return Err(Impossible);
            }
            let y = q.checked_mul(100).and_then(|v| v.checked_add(r));
            Ok(Some(y.ok_or(OutOfRange)?))
        }
        (None, None, Some(r @ 0..=99)) => Ok(Some(r + if r < 70 { 2000 } else { 1900 })),
        (None, Some(_), None) => Err(NotEnough),
        (_, _, Some(_)) => Err(OutOfRange),
    }
}

/// A day of the week, as days since Monday.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Weekday(u8);

impl Weekday {
    const MONDAY: Self = Self(0);
    const SUNDAY: Self = Self(6);

    /// Days from `other` forward to `self`, 0 to 6.
    fn days_since(self, other: Self) -> u32 {
        u32::from((self.0 + 7 - other.0) % 7)
    }
}

/// A day in the years -262143 to 262142, as days since 1970-01-01.
#[derive(Clone, Copy)]
struct Date {
    days: i64,
}

impl Date {
    fn in_range(days: i64) -> bool {
        (days_from_civil(MIN_YEAR, 1, 1)..=days_from_civil(MAX_YEAR, 12, 31)).contains(&days)
    }

    fn checked(days: i64) -> Option<Self> {
        Self::in_range(days).then_some(Self { days })
    }

    fn from_ymd(year: i32, month: u32, day: u32) -> Option<Self> {
        let valid = (MIN_YEAR..=MAX_YEAR).contains(&year)
            && (1..=12).contains(&month)
            && day >= 1
            && day <= days_in_month(year, month as i8) as u32;
        valid.then(|| Self {
            days: days_from_civil(year, month as i8, day as i8),
        })
    }

    /// Day `ordinal` of `year`, counting January 1 as 1.
    fn from_yo(year: i32, ordinal: u32) -> Option<Self> {
        let valid =
            (MIN_YEAR..=MAX_YEAR).contains(&year) && ordinal >= 1 && ordinal <= days_in_year(year);
        valid.then(|| Self {
            days: days_from_civil(year, 1, 1) + i64::from(ordinal) - 1,
        })
    }

    /// The day of ISO week `week` of `isoyear` that falls on `weekday`.
    fn from_isoywd(isoyear: i32, week: u32, weekday: Weekday) -> Option<Self> {
        if week == 0 || week > iso_weeks(isoyear) {
            return None;
        }
        let january_4 = Self {
            days: days_from_civil(isoyear, 1, 4),
        };
        let monday = january_4.days - i64::from(january_4.weekday().0);
        Self::checked(monday + i64::from(week - 1) * 7 + i64::from(weekday.0))
    }

    fn year_month_day(self) -> (i32, u32, u32) {
        let (year, month, day) = civil_from_days(self.days);
        (year, month as u32, day as u32)
    }

    /// The day of the year, counting January 1 as 1.
    fn ordinal(self) -> u32 {
        let (year, _, _) = civil_from_days(self.days);
        (self.days - days_from_civil(year, 1, 1) + 1) as u32
    }

    fn weekday(self) -> Weekday {
        // 1970-01-01 was a Thursday.
        Weekday((self.days + 3).rem_euclid(7) as u8)
    }

    /// The ISO week-numbering year and week.
    fn iso_week(self) -> (i32, u32) {
        // The ISO week belongs to the year of its Thursday.
        let thursday = Self {
            days: self.days - i64::from(self.weekday().0) + 3,
        };
        let (year, _, _) = civil_from_days(thursday.days);
        (year, (thursday.ordinal() - 1) / 7 + 1)
    }

    /// The week of the year, where week 1 starts on the first `start` day and the days before
    /// it are week 0, as `%U` (Sunday) and `%W` (Monday) count them.
    fn weeks_from(self, start: Weekday) -> i32 {
        (self.ordinal() as i32 - self.weekday().days_since(start) as i32 + 6) / 7
    }

    /// This date at `time`, at `offset`.
    fn at(self, time: TimeOfDay, offset: Offset) -> Option<DateTime> {
        let (year, month, day) = civil_from_days(self.days);
        DateTime::from_parts(
            year,
            month,
            day,
            time.hour,
            time.minute,
            time.second,
            time.nanosecond,
            offset,
        )
    }
}

fn days_in_year(year: i32) -> u32 {
    if days_in_month(year, 2) == 29 {
        366
    } else {
        365
    }
}

/// How many ISO weeks `isoyear` has: 52, or 53 when December 28 is in week 53.
fn iso_weeks(isoyear: i32) -> u32 {
    Date {
        days: days_from_civil(isoyear, 12, 28),
    }
    .iso_week()
    .1
}

/// The date of `weekday` in week `week` of `year`, where week 1 starts on the first
/// `week_start` day.
fn resolve_week_date(
    year: i32,
    week: u32,
    weekday: Weekday,
    week_start: Weekday,
) -> ParseResult<Date> {
    if week > 53 {
        return Err(OutOfRange);
    }
    let first_day = Date::from_yo(year, 1).ok_or(OutOfRange)?;
    let first_week_start = 1 + week_start.days_since(first_day.weekday()) as i32;
    let ordinal = first_week_start + (week as i32 - 1) * 7 + weekday.days_since(week_start) as i32;
    if ordinal <= 0 {
        return Err(Impossible);
    }
    Date::from_yo(year, ordinal as u32).ok_or(Impossible)
}

/// One piece of a format.
#[derive(Clone, Copy)]
enum Item<'a> {
    /// Text that must appear as is.
    Literal(&'a str),
    /// Whitespace in the format, which matches any amount of whitespace, or none.
    Space,
    Numeric(Numeric),
    Fixed(Fixed),
    /// A directive that isn't supported; reading fails here.
    Error,
}

/// A number field, read as up to [`Numeric::width`] digits after any whitespace.
#[derive(Clone, Copy)]
enum Numeric {
    Year,
    YearDiv100,
    YearMod100,
    IsoYear,
    IsoYearMod100,
    Quarter,
    Month,
    Day,
    WeekFromSun,
    WeekFromMon,
    IsoWeek,
    NumDaysFromSun,
    WeekdayFromMon,
    Ordinal,
    Hour,
    Hour12,
    Minute,
    Second,
    Nanosecond,
    Timestamp,
}

impl Numeric {
    /// The most digits read, and whether a sign is allowed. A signed number may have any
    /// number of digits.
    fn width(self) -> (usize, bool) {
        match self {
            Self::Year | Self::IsoYear => (4, true),
            Self::Quarter | Self::NumDaysFromSun | Self::WeekdayFromMon => (1, false),
            Self::Ordinal => (3, false),
            Self::Nanosecond => (9, false),
            // chrono read `%s` unsigned, so a time before 1970 couldn't be read back.
            Self::Timestamp => (usize::MAX, true),
            _ => (2, false),
        }
    }
}

/// A field read by its own rule rather than as a number.
#[derive(Clone, Copy)]
enum Fixed {
    ShortMonthName,
    LongMonthName,
    ShortWeekdayName,
    LongWeekdayName,
    AmPm,
    /// `%.f`: an optional `.` and 1 to 9 digits of fraction, any further digits ignored.
    Fraction,
    /// `%.3f`, `%.6f`, `%.9f`: an optional `.` and exactly that many digits.
    FractionDigits(usize),
    /// `%3f`, `%6f`, `%9f`: exactly that many digits.
    FractionNoDot(usize),
    /// `%Z`: `UTC` or `GMT` in any case, which set offset zero, or an offset such as `+05:30`,
    /// which is how nushell writes `%Z`. chrono skipped any word here and left the offset unset,
    /// so `PST` read as local time; other names are an error now, since abbreviations such as
    /// `CST` name more than one offset.
    TimezoneName,
    /// `%z`, `%:z`, `%::z`, `%:::z`: `+08:00`, `+0800` or `+08 00`.
    Offset,
    /// `%#z`: also `Z` and an offset without minutes.
    OffsetPermissive,
    /// `%+`: RFC 3339, with `T`, `t` or a space between date and time, and `Z` or `UTC`
    /// for the offset.
    Rfc3339,
}

const D_FMT: &[Item<'static>] = &[
    Item::Numeric(Numeric::Month),
    Item::Literal("/"),
    Item::Numeric(Numeric::Day),
    Item::Literal("/"),
    Item::Numeric(Numeric::YearMod100),
];
const F_FMT: &[Item<'static>] = &[
    Item::Numeric(Numeric::Year),
    Item::Literal("-"),
    Item::Numeric(Numeric::Month),
    Item::Literal("-"),
    Item::Numeric(Numeric::Day),
];
/// nushell's `%J`, a joined date (`%Y%m%d`).
const J_FMT: &[Item<'static>] = &[
    Item::Numeric(Numeric::Year),
    Item::Numeric(Numeric::Month),
    Item::Numeric(Numeric::Day),
];
/// nushell's `%Q`, a joined time (`%H%M%S`).
const Q_FMT: &[Item<'static>] = &[
    Item::Numeric(Numeric::Hour),
    Item::Numeric(Numeric::Minute),
    Item::Numeric(Numeric::Second),
];
const R_FMT: &[Item<'static>] = &[
    Item::Numeric(Numeric::Hour),
    Item::Literal(":"),
    Item::Numeric(Numeric::Minute),
];
const T_FMT: &[Item<'static>] = &[
    Item::Numeric(Numeric::Hour),
    Item::Literal(":"),
    Item::Numeric(Numeric::Minute),
    Item::Literal(":"),
    Item::Numeric(Numeric::Second),
];
const T_FMT_AMPM: &[Item<'static>] = &[
    Item::Numeric(Numeric::Hour12),
    Item::Literal(":"),
    Item::Numeric(Numeric::Minute),
    Item::Literal(":"),
    Item::Numeric(Numeric::Second),
    Item::Space,
    Item::Fixed(Fixed::AmPm),
];
const V_FMT: &[Item<'static>] = &[
    Item::Numeric(Numeric::Day),
    Item::Literal("-"),
    Item::Fixed(Fixed::ShortMonthName),
    Item::Literal("-"),
    Item::Numeric(Numeric::Year),
];
const D_T_FMT: &[Item<'static>] = &[
    Item::Fixed(Fixed::ShortWeekdayName),
    Item::Space,
    Item::Fixed(Fixed::ShortMonthName),
    Item::Space,
    Item::Numeric(Numeric::Day),
    Item::Space,
    Item::Numeric(Numeric::Hour),
    Item::Literal(":"),
    Item::Numeric(Numeric::Minute),
    Item::Literal(":"),
    Item::Numeric(Numeric::Second),
    Item::Space,
    Item::Numeric(Numeric::Year),
];
const RFC3339_DATE: &[Item<'static>] = &[
    Item::Numeric(Numeric::Year),
    Item::Space,
    Item::Literal("-"),
    Item::Numeric(Numeric::Month),
    Item::Space,
    Item::Literal("-"),
    Item::Numeric(Numeric::Day),
];
const RFC3339_TIME: &[Item<'static>] = &[
    Item::Numeric(Numeric::Hour),
    Item::Space,
    Item::Literal(":"),
    Item::Numeric(Numeric::Minute),
    Item::Space,
    Item::Literal(":"),
    Item::Numeric(Numeric::Second),
    Item::Fixed(Fixed::Fraction),
    Item::Space,
];

/// Splits `format` into items, stopping after the first unsupported directive.
fn items(format: &str) -> Vec<Item<'_>> {
    let mut items = Vec::new();
    let mut rest = format;
    while let Some(c) = rest.chars().next() {
        if c != '%' {
            let end = if c.is_whitespace() {
                rest.find(|c: char| !c.is_whitespace())
            } else {
                rest.find(|c: char| c.is_whitespace() || c == '%')
            }
            .unwrap_or(rest.len());
            items.push(if c.is_whitespace() {
                Item::Space
            } else {
                Item::Literal(&rest[..end])
            });
            rest = &rest[end..];
            continue;
        }
        let mut chars = rest[1..].chars();
        match directive(&mut chars) {
            Some(Directive::One(item)) => items.push(item),
            Some(Directive::Many(expansion)) => items.extend_from_slice(expansion),
            None => {
                items.push(Item::Error);
                break;
            }
        }
        rest = chars.as_str();
    }
    items
}

/// What one `%` directive stands for.
enum Directive {
    One(Item<'static>),
    Many(&'static [Item<'static>]),
}

/// Reads the directive after a `%`, or `None` when it isn't supported. A padding flag (`-`,
/// `0` or `_`) is allowed only on a number field, and `#` only on `z`.
fn directive(chars: &mut std::str::Chars<'_>) -> Option<Directive> {
    let first = chars.next()?;
    let padded = matches!(first, '-' | '0' | '_');
    let alternate = first == '#';
    let spec = if padded || alternate {
        chars.next()?
    } else {
        first
    };
    if alternate && spec != 'z' {
        return None;
    }
    let numeric = |numeric| Directive::One(Item::Numeric(numeric));
    let fixed = |fixed| Directive::One(Item::Fixed(fixed));
    let directive = match spec {
        'A' => fixed(Fixed::LongWeekdayName),
        'B' => fixed(Fixed::LongMonthName),
        'C' => numeric(Numeric::YearDiv100),
        'D' | 'x' => Directive::Many(D_FMT),
        'F' => Directive::Many(F_FMT),
        'G' => numeric(Numeric::IsoYear),
        'H' | 'k' => numeric(Numeric::Hour),
        'I' | 'l' => numeric(Numeric::Hour12),
        'J' => Directive::Many(J_FMT),
        'M' => numeric(Numeric::Minute),
        'P' | 'p' => fixed(Fixed::AmPm),
        'Q' => Directive::Many(Q_FMT),
        'R' => Directive::Many(R_FMT),
        'S' => numeric(Numeric::Second),
        'T' | 'X' => Directive::Many(T_FMT),
        'U' => numeric(Numeric::WeekFromSun),
        'V' => numeric(Numeric::IsoWeek),
        'W' => numeric(Numeric::WeekFromMon),
        'Y' => numeric(Numeric::Year),
        'Z' => fixed(Fixed::TimezoneName),
        'a' => fixed(Fixed::ShortWeekdayName),
        'b' | 'h' => fixed(Fixed::ShortMonthName),
        'c' => Directive::Many(D_T_FMT),
        'd' | 'e' => numeric(Numeric::Day),
        'f' => numeric(Numeric::Nanosecond),
        'g' => numeric(Numeric::IsoYearMod100),
        'j' => numeric(Numeric::Ordinal),
        'm' => numeric(Numeric::Month),
        'n' | 't' => Directive::One(Item::Space),
        'q' => numeric(Numeric::Quarter),
        'r' => Directive::Many(T_FMT_AMPM),
        's' => numeric(Numeric::Timestamp),
        'u' => numeric(Numeric::WeekdayFromMon),
        'v' => Directive::Many(V_FMT),
        'w' => numeric(Numeric::NumDaysFromSun),
        'y' => numeric(Numeric::YearMod100),
        'z' if alternate => fixed(Fixed::OffsetPermissive),
        'z' => fixed(Fixed::Offset),
        '+' => fixed(Fixed::Rfc3339),
        ':' => {
            let rest = chars.as_str();
            let skip = ["::z", ":z", "z"]
                .iter()
                .find(|form| rest.starts_with(*form))?
                .len();
            *chars = rest[skip..].chars();
            fixed(Fixed::Offset)
        }
        '.' => match chars.next()? {
            'f' => fixed(Fixed::Fraction),
            digits @ ('3' | '6' | '9') if chars.next()? == 'f' => {
                fixed(Fixed::FractionDigits(digit_count(digits)))
            }
            _ => return None,
        },
        digits @ ('3' | '6' | '9') if chars.next()? == 'f' => {
            fixed(Fixed::FractionNoDot(digit_count(digits)))
        }
        '%' => Directive::One(Item::Literal("%")),
        _ => return None,
    };
    match directive {
        Directive::One(Item::Numeric(_)) => Some(directive),
        _ if padded => None,
        _ => Some(directive),
    }
}

fn digit_count(digit: char) -> usize {
    digit.to_digit(10).map_or(0, |d| d as usize)
}

/// Reads `s` item by item into `parsed`, returning the text left over.
fn parse_items<'a>(
    parsed: &mut ParsedDate,
    mut s: &'a str,
    items: &[Item<'_>],
) -> ParseResult<&'a str> {
    for item in items {
        match *item {
            Item::Literal(prefix) => {
                if s.len() < prefix.len() {
                    return Err(TooShort);
                }
                s = s.strip_prefix(prefix).ok_or(Invalid)?;
            }
            Item::Space => s = s.trim_start(),
            Item::Numeric(numeric) => {
                let (width, signed) = numeric.width();
                s = s.trim_start();
                let value = match s.strip_prefix(['-', '+']) {
                    Some(digits) if signed => {
                        let (rest, value) = number(digits, 1, usize::MAX)?;
                        let negative = s.starts_with('-');
                        if negative && matches!(numeric, Numeric::Timestamp) {
                            parsed.timestamp_negative = true;
                        }
                        s = rest;
                        if negative {
                            0i64.checked_sub(value).ok_or(OutOfRange)?
                        } else {
                            value
                        }
                    }
                    _ => {
                        let (rest, value) = number(s, 1, width)?;
                        s = rest;
                        value
                    }
                };
                parsed.set_numeric(numeric, value)?;
            }
            Item::Fixed(fixed) => s = parse_fixed(parsed, s, fixed)?,
            Item::Error => return Err(BadFormat),
        }
    }
    Ok(s)
}

fn parse_fixed<'a>(parsed: &mut ParsedDate, s: &'a str, fixed: Fixed) -> ParseResult<&'a str> {
    Ok(match fixed {
        Fixed::ShortMonthName => {
            let (rest, month0) = short_name(s, &MONTHS)?;
            parsed.set_month(month0 as i64 + 1)?;
            rest
        }
        Fixed::LongMonthName => {
            let (rest, month0) = short_or_long_name(s, &MONTHS)?;
            parsed.set_month(month0 as i64 + 1)?;
            rest
        }
        Fixed::ShortWeekdayName => {
            let (rest, day) = short_name(s, &WEEKDAYS)?;
            parsed.set_weekday(Weekday(day as u8))?;
            rest
        }
        Fixed::LongWeekdayName => {
            let (rest, day) = short_or_long_name(s, &WEEKDAYS)?;
            parsed.set_weekday(Weekday(day as u8))?;
            rest
        }
        Fixed::AmPm => {
            let bytes = s.as_bytes();
            if bytes.len() < 2 {
                return Err(TooShort);
            }
            let pm = match (bytes[0] | 32, bytes[1] | 32) {
                (b'a', b'm') => false,
                (b'p', b'm') => true,
                _ => return Err(Invalid),
            };
            set_if_consistent(&mut parsed.hour_div_12, u32::from(pm))?;
            &s[2..]
        }
        Fixed::Fraction => match s.strip_prefix('.') {
            Some(digits) => {
                let (rest, nanos) = nanosecond(digits)?;
                parsed.set_nanosecond(nanos)?;
                rest
            }
            None => s,
        },
        Fixed::FractionDigits(digits) => match s.strip_prefix('.') {
            Some(text) => {
                let (rest, nanos) = nanosecond_fixed(text, digits)?;
                parsed.set_nanosecond(nanos)?;
                rest
            }
            None => s,
        },
        Fixed::FractionNoDot(digits) => {
            if s.len() < digits {
                return Err(TooShort);
            }
            let (rest, nanos) = nanosecond_fixed(s, digits)?;
            parsed.set_nanosecond(nanos)?;
            rest
        }
        Fixed::TimezoneName => {
            let s = s.trim_start();
            let (rest, offset) = match timezone_offset(s, false, false) {
                Ok(read) => read,
                Err(_) => {
                    let (name, rest) =
                        s.split_at(s.bytes().take_while(u8::is_ascii_alphabetic).count());
                    if !(name.eq_ignore_ascii_case("UTC") || name.eq_ignore_ascii_case("GMT")) {
                        return Err(Invalid);
                    }
                    (rest, 0)
                }
            };
            parsed.set_offset(offset)?;
            rest
        }
        Fixed::Offset | Fixed::OffsetPermissive => {
            let permissive = matches!(fixed, Fixed::OffsetPermissive);
            let (rest, offset) = timezone_offset(s.trim_start(), permissive, permissive)?;
            parsed.set_offset(offset)?;
            rest
        }
        Fixed::Rfc3339 => parse_rfc3339_relaxed(parsed, s)?,
    })
}

/// `%+`: chrono's relaxed RFC 3339.
fn parse_rfc3339_relaxed<'a>(parsed: &mut ParsedDate, s: &'a str) -> ParseResult<&'a str> {
    let s = parse_items(parsed, s, RFC3339_DATE)?;
    let s = match s.as_bytes().first() {
        Some(b't' | b'T' | b' ') => &s[1..],
        Some(_) => return Err(Invalid),
        None => return Err(TooShort),
    };
    let s = parse_items(parsed, s, RFC3339_TIME)?.trim_start();
    let (s, offset) = if s.len() >= 3 && s.as_bytes()[..3].eq_ignore_ascii_case(b"UTC") {
        (&s[3..], 0)
    } else {
        timezone_offset(s, true, false)?
    };
    parsed.set_offset(offset)?;
    Ok(s)
}

/// Reads `min` to `max` ASCII digits as a number.
fn number(s: &str, min: usize, max: usize) -> ParseResult<(&str, i64)> {
    let bytes = s.as_bytes();
    if bytes.len() < min {
        return Err(TooShort);
    }
    let mut n = 0i64;
    for (i, &c) in bytes.iter().take(max).enumerate() {
        if !c.is_ascii_digit() {
            return if i < min {
                Err(Invalid)
            } else {
                Ok((&s[i..], n))
            };
        }
        n = n
            .checked_mul(10)
            .and_then(|n| n.checked_add(i64::from(c - b'0')))
            .ok_or(OutOfRange)?;
    }
    Ok((&s[max.min(bytes.len())..], n))
}

const SCALE: [i64; 10] = [
    0,
    100_000_000,
    10_000_000,
    1_000_000,
    100_000,
    10_000,
    1_000,
    100,
    10,
    1,
];

/// 1 to 9 digits of a fraction of a second, as nanoseconds; further digits are skipped.
fn nanosecond(s: &str) -> ParseResult<(&str, i64)> {
    let (rest, value) = number(s, 1, 9)?;
    let nanos = value
        .checked_mul(SCALE[s.len() - rest.len()])
        .ok_or(OutOfRange)?;
    Ok((rest.trim_start_matches(|c: char| c.is_ascii_digit()), nanos))
}

/// Exactly `digits` digits of a fraction of a second, as nanoseconds.
fn nanosecond_fixed(s: &str, digits: usize) -> ParseResult<(&str, i64)> {
    let (rest, value) = number(s, digits, digits)?;
    Ok((rest, value.checked_mul(SCALE[digits]).ok_or(OutOfRange)?))
}

const MONTHS: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];
const WEEKDAYS: [&str; 7] = [
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
];

/// The first three letters of one of `names`, ignoring ASCII case.
fn short_name<'a>(s: &'a str, names: &[&str]) -> ParseResult<(&'a str, usize)> {
    let bytes = s.as_bytes();
    if bytes.len() < 3 {
        return Err(TooShort);
    }
    names
        .iter()
        .position(|name| bytes[..3].eq_ignore_ascii_case(&name.as_bytes()[..3]))
        .map(|index| (&s[3..], index))
        .ok_or(Invalid)
}

/// One of `names`, in full or as its first three letters, ignoring ASCII case.
fn short_or_long_name<'a>(s: &'a str, names: &[&str]) -> ParseResult<(&'a str, usize)> {
    let (rest, index) = short_name(s, names)?;
    let suffix = &names[index].as_bytes()[3..];
    let bytes = rest.as_bytes();
    if bytes.len() >= suffix.len() && bytes[..suffix.len()].eq_ignore_ascii_case(suffix) {
        Ok((&rest[suffix.len()..], index))
    } else {
        Ok((rest, index))
    }
}

/// An offset: a sign (`+`, `-` or `−`), two hour digits, any colons and whitespace, and two
/// minute digits. `allow_zulu` also reads `Z`, and `allow_missing_minutes` an hour alone.
/// Returns the text left over and the offset in seconds east of UTC.
fn timezone_offset(
    s: &str,
    allow_zulu: bool,
    allow_missing_minutes: bool,
) -> ParseResult<(&str, i32)> {
    if allow_zulu && s.starts_with(['Z', 'z']) {
        return Ok((&s[1..], 0));
    }
    let two_digits = |s: &str| match s.as_bytes() {
        [a, b, ..] => Ok((*a, *b)),
        _ => Err(TooShort),
    };
    let (negative, s) = match s.chars().next() {
        Some('+') => (false, &s[1..]),
        Some('-') => (true, &s[1..]),
        Some('\u{2212}') => (true, &s['\u{2212}'.len_utf8()..]),
        Some(_) => return Err(Invalid),
        None => return Err(TooShort),
    };
    let hours = match two_digits(s)? {
        (h1 @ b'0'..=b'9', h2 @ b'0'..=b'9') => i32::from((h1 - b'0') * 10 + (h2 - b'0')),
        _ => return Err(Invalid),
    };
    let s = s[2..].trim_start_matches(|c: char| c == ':' || c.is_whitespace());
    let minutes = match two_digits(s) {
        Ok((m1 @ b'0'..=b'5', m2 @ b'0'..=b'9')) => i32::from((m1 - b'0') * 10 + (m2 - b'0')),
        Ok((b'6'..=b'9', b'0'..=b'9')) => return Err(OutOfRange),
        Ok(_) => return Err(Invalid),
        Err(_) if allow_missing_minutes => 0,
        Err(_) => return Err(TooShort),
    };
    let s = match s.len() {
        len if len >= 2 => &s[2..],
        0 => s,
        _ => return Err(TooShort),
    };
    let seconds = hours * 3600 + minutes * 60;
    Ok((s, if negative { -seconds } else { seconds }))
}
