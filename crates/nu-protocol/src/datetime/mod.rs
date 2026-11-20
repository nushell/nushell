//! Nushell's date value: an instant, and the UTC offset it is shown in.
//!
//! [`DateTime`] stores the instant itself, over the years chrono covered (-262143 to 262142),
//! with leap seconds as chrono kept them, and uses [`jiff`] for time zones and `strftime`. Its
//! text forms (`Display`, `Debug`, RFC 3339, RFC 2822, serde and `strftime` output) match what
//! nushell printed when dates were `chrono::DateTime<FixedOffset>`, so scripts, saved files and
//! plugins see the same text.

#[cfg(feature = "chrono")]
mod chrono_compat;
mod format;
mod humanize;
mod parse;
mod strptime;

pub use format::DateFormatError;
pub use humanize::HumanTime;
pub use parse::{DateParseError, parse_utc_offset};
pub use strptime::{ParsedDate, TimeOfDay};

use jiff::{
    Timestamp, ZonedDateTime, civil,
    tz::{AmbiguousOffset, Disambiguation, Offset, TimeZone},
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
    str::FromStr,
    time::SystemTime,
};

/// The `LC_TIME` locale named by the environment, read through `env`, or `en_US` when the name
/// isn't one pure-rust-locales knows.
pub fn time_locale<'a, F, O>(env: F) -> pure_rust_locales::Locale
where
    F: 'a + Fn(&str) -> Option<O>,
    O: Into<std::borrow::Cow<'a, str>>,
{
    nu_utils::get_locale_from_env_vars(Some("LC_TIME"), env)
        .and_then(|name| pure_rust_locales::Locale::try_from(name.as_ref()).ok())
        .unwrap_or(pure_rust_locales::Locale::en_US)
}

/// The first and last years a date can have, as with chrono.
const MIN_YEAR: i32 = -262_143;
const MAX_YEAR: i32 = 262_142;
const NANOS_PER_SECOND: u32 = 1_000_000_000;
const SECONDS_PER_DAY: i64 = 86_400;

/// An instant in time together with the UTC offset it is displayed in.
///
/// Two dates are equal, ordered and hashed by their instant alone, so
/// `2001-02-03T04:05:06+05:30 == 2001-02-02T22:35:06Z`.
#[derive(Clone, Copy)]
pub struct DateTime {
    /// Seconds since the Unix epoch, rounded down.
    seconds: i64,
    /// Nanoseconds after `seconds`. A leap second, which chrono kept on second 59 of its minute,
    /// has 1,000,000,000 or more.
    nanos: u32,
    offset: Offset,
}

/// The wall-clock fields of a [`DateTime`] at its offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DateParts {
    pub year: i32,
    pub month: i8,
    pub day: i8,
    pub hour: i8,
    pub minute: i8,
    /// 0 to 59; a leap second is second 59 with a `nanosecond` of 1,000,000,000 or more.
    pub second: i8,
    pub nanosecond: i32,
    /// Days since Sunday, 0 to 6.
    pub weekday: i8,
}

impl DateParts {
    /// Whether this is a leap second (`23:59:60`).
    pub fn is_leap_second(self) -> bool {
        self.nanosecond >= NANOS_PER_SECOND as i32
    }
}

/// How many fraction digits [`DateTime::to_rfc3339_opts`] writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecondsFormat {
    /// No fraction.
    Secs,
    /// Exactly 3 digits.
    Millis,
    /// Exactly 6 digits.
    Micros,
    /// Exactly 9 digits.
    Nanos,
    /// None when the fraction is zero, otherwise the shortest of 3, 6 or 9 digits.
    AutoSi,
}

impl DateTime {
    /// 1970-01-01T00:00:00Z.
    pub const UNIX_EPOCH: Self = Self {
        seconds: 0,
        nanos: 0,
        offset: Offset::UTC,
    };

    /// The date at `seconds` + `nanos` after the epoch, at `offset`, if it is within the years
    /// -262143 to 262142 both at UTC and at the offset.
    fn checked(seconds: i64, nanos: u32, offset: Offset) -> Option<Self> {
        let in_range = |seconds: i64| {
            let days = seconds.div_euclid(SECONDS_PER_DAY);
            (days_from_civil(MIN_YEAR, 1, 1)..=days_from_civil(MAX_YEAR, 12, 31)).contains(&days)
        };
        let local = seconds.checked_add(offset.seconds().into())?;
        (in_range(seconds) && in_range(local) && nanos < 2 * NANOS_PER_SECOND).then_some(Self {
            seconds,
            nanos,
            offset,
        })
    }

    /// The date at `timestamp`, shown at `offset`.
    pub fn new(timestamp: Timestamp, offset: Offset) -> Self {
        let nanos = timestamp.as_nanosecond();
        Self {
            seconds: nanos.div_euclid(NANOS_PER_SECOND.into()) as i64,
            nanos: nanos.rem_euclid(NANOS_PER_SECOND.into()) as u32,
            offset,
        }
    }

    /// The instant and offset of a jiff zoned date-time. The time zone itself is dropped.
    pub fn from_zoned(zoned: &ZonedDateTime) -> Self {
        Self::new(zoned.timestamp(), zoned.offset())
    }

    /// The wall-clock `datetime` at `offset`.
    pub fn from_civil(datetime: civil::DateTime, offset: Offset) -> Option<Self> {
        Self::from_parts(
            datetime.year().into(),
            datetime.month(),
            datetime.day(),
            datetime.hour(),
            datetime.minute(),
            datetime.second(),
            datetime.subsec_nanosecond(),
            offset,
        )
    }

    /// The wall-clock `datetime` in `time_zone`, with `disambiguation` picking the instant when
    /// the zone skips or repeats that time; `None` when it rejects the time. A time the zone
    /// skips comes out as the instant `disambiguation` picks, so its wall-clock time moves, as
    /// `02:30` becomes `03:30` when clocks go forward at 02:00. Near the ends of years -9999 and
    /// 9999, which jiff's timestamps don't reach, the same wall-clock time is read 400 years
    /// closer, where the calendar and a zone's yearly rules repeat, and moved back.
    pub fn from_civil_in(
        datetime: civil::DateTime,
        time_zone: &TimeZone,
        disambiguation: Disambiguation,
    ) -> Option<Self> {
        let resolve = |datetime: civil::DateTime| {
            let timestamp = time_zone
                .to_ambiguous_timestamp(datetime)
                .disambiguate(disambiguation)
                .ok()?;
            Some(Self::new(timestamp, time_zone.to_offset(timestamp)))
        };
        if let Some(date) = resolve(datetime) {
            return Some(date);
        }
        if datetime.year().unsigned_abs() < 9998 {
            return None;
        }
        let years = if datetime.year() > 0 { -400 } else { 400 };
        let closer = datetime.with().year(datetime.year() + years).build().ok()?;
        resolve(closer)?.add_years(-i32::from(years))
    }

    /// The instants the wall-clock `datetime` names in `time_zone`, earlier first, as chrono's
    /// `Local` found them for the system time zone: none in a skipped hour, two in a repeated
    /// one. chrono kept an offset valid for one second past the instant it ended, so the first
    /// second of a skipped hour reads at the old offset (`02:00` stays `02:00-05:00`), and the
    /// first second after a repeated hour has two readings. Empty when jiff can't place the time.
    pub fn chrono_local_readings(datetime: civil::DateTime, time_zone: &TimeZone) -> Vec<Self> {
        let second = jiff::SignedDuration::from_secs(1);
        let at = |offset: Offset| offset.to_timestamp(datetime).ok();
        let mut offsets = match time_zone.to_ambiguous_timestamp(datetime).offset() {
            AmbiguousOffset::Unambiguous { offset } => vec![offset],
            AmbiguousOffset::Gap { before, after } | AmbiguousOffset::Fold { before, after } => {
                vec![before, after]
            }
        };
        // Every offset the zone has within a day of the time, since chrono could still allow
        // one that had just ended.
        if let Some(probe) = at(Offset::UTC) {
            for transition in time_zone.preceding(probe).take(2) {
                offsets.push(transition.offset());
                if let Ok(before) = transition.timestamp().checked_sub(second) {
                    offsets.push(time_zone.to_offset(before));
                }
            }
            offsets.extend(time_zone.following(probe).take(2).map(|t| t.offset()));
        }
        let mut readings: Vec<Self> = Vec::new();
        for offset in offsets {
            let Some(timestamp) = at(offset) else {
                continue;
            };
            let valid = time_zone.to_offset(timestamp) == offset
                || timestamp
                    .checked_sub(second)
                    .is_ok_and(|earlier| time_zone.to_offset(earlier) == offset);
            if valid && !readings.iter().any(|r| r.timestamp() == Some(timestamp)) {
                readings.push(Self::new(timestamp, offset));
            }
        }
        readings.sort();
        readings
    }

    /// The same wall-clock time and offset `years` later, or `None` when that day doesn't exist.
    fn add_years(self, years: i32) -> Option<Self> {
        let p = self.parts();
        Self::from_parts(
            p.year.checked_add(years)?,
            p.month,
            p.day,
            p.hour,
            p.minute,
            p.second,
            p.nanosecond,
            self.offset,
        )
    }

    /// The wall-clock date and time at `offset`, or `None` when a field is out of range or the
    /// year is outside -262143 to 262142. A `nanosecond` of 1,000,000,000 or more on second 59
    /// is a leap second.
    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        year: i32,
        month: i8,
        day: i8,
        hour: i8,
        minute: i8,
        second: i8,
        nanosecond: i32,
        offset: Offset,
    ) -> Option<Self> {
        let leap = nanosecond >= NANOS_PER_SECOND as i32;
        let valid = (MIN_YEAR..=MAX_YEAR).contains(&year)
            && (1..=12).contains(&month)
            && (1..=days_in_month(year, month)).contains(&day)
            && (0..24).contains(&hour)
            && (0..60).contains(&minute)
            && (0..60).contains(&second)
            && (0..2 * NANOS_PER_SECOND as i32).contains(&nanosecond)
            && (!leap || second == 59);
        if !valid {
            return None;
        }
        let local = days_from_civil(year, month, day) * SECONDS_PER_DAY
            + i64::from(hour) * 3600
            + i64::from(minute) * 60
            + i64::from(second);
        Self::checked(
            local - i64::from(offset.seconds()),
            nanosecond as u32,
            offset,
        )
    }

    /// The instant this date's wall-clock fields name in `time_zone`, with `disambiguation`
    /// picking the offset when the zone skips or repeats that time; `None` when it rejects the
    /// time. The offset this date has is ignored. For years outside -9998 to 9998 the zone's
    /// offset is read 400-year cycles closer, where the calendar and a zone's yearly rules
    /// repeat. A leap second stays one.
    pub fn wall_clock_in(
        self,
        time_zone: &TimeZone,
        disambiguation: Disambiguation,
    ) -> Option<Self> {
        let p = self.parts();
        let held = if (-9998..=9998).contains(&p.year) {
            p.year
        } else {
            2000 + (p.year - 2000).rem_euclid(400)
        };
        let civil = civil::DateTime::new(
            i16::try_from(held).ok()?,
            p.month,
            p.day,
            p.hour,
            p.minute,
            p.second,
            p.nanosecond % NANOS_PER_SECOND as i32,
        )
        .ok()?;
        let date =
            Self::from_civil_in(civil, time_zone, disambiguation)?.add_years(p.year - held)?;
        if !p.is_leap_second() {
            return Some(date);
        }
        // jiff has no leap seconds: put it back, unless the zone's offset moves second 59 off
        // UTC's.
        Some(
            Self::from_unix_timestamp(date.seconds, date.nanos + NANOS_PER_SECOND, date.offset)
                .unwrap_or(date),
        )
    }

    /// The current time in the system time zone.
    pub fn now() -> Self {
        Self::from_zoned(&ZonedDateTime::now())
    }

    /// The current time at UTC.
    pub fn now_utc() -> Self {
        Self::new(Timestamp::now(), Offset::UTC)
    }

    /// `timestamp`, shown at the system time zone's offset for that instant.
    pub fn from_timestamp_local(timestamp: Timestamp) -> Self {
        Self::new(timestamp, TimeZone::system().to_offset(timestamp))
    }

    /// Nanoseconds since the Unix epoch, shown at `offset`.
    pub fn from_unix_nanos(nanos: i128, offset: Offset) -> Option<Self> {
        let seconds = i64::try_from(nanos.div_euclid(NANOS_PER_SECOND.into())).ok()?;
        Self::checked(
            seconds,
            nanos.rem_euclid(NANOS_PER_SECOND.into()) as u32,
            offset,
        )
    }

    /// Milliseconds since the Unix epoch, shown at `offset`.
    pub fn from_unix_millis(millis: i64, offset: Offset) -> Option<Self> {
        Self::from_unix_nanos(i128::from(millis) * 1_000_000, offset)
    }

    /// Whole seconds since the Unix epoch, rounded down, and the nanoseconds after them, shown at
    /// `offset`, as chrono's `DateTime::from_timestamp` took them: `nanos` of 1,000,000,000 or
    /// more is a leap second, allowed only on second 59 of a minute.
    pub fn from_unix_timestamp(seconds: i64, nanos: u32, offset: Offset) -> Option<Self> {
        if nanos >= NANOS_PER_SECOND && seconds.rem_euclid(60) != 59 {
            return None;
        }
        Self::checked(seconds, nanos, offset)
    }

    /// Seconds and nanoseconds since the Unix epoch, shown at `offset`. `nanos` may be negative,
    /// as jiff gives it for instants before the epoch.
    pub fn from_unix_seconds(seconds: i64, nanos: i32, offset: Offset) -> Option<Self> {
        Self::from_unix_nanos(
            i128::from(seconds) * i128::from(NANOS_PER_SECOND) + i128::from(nanos),
            offset,
        )
    }

    /// A file or clock time, shown in the system time zone.
    pub fn from_system_time(time: SystemTime) -> Option<Self> {
        Timestamp::try_from(time)
            .ok()
            .map(Self::from_timestamp_local)
    }

    /// The instant as a jiff timestamp, or `None` near and past the ends of years -9999 and
    /// 9999, which jiff's timestamps don't reach. A leap second becomes its minute's last
    /// nanosecond.
    pub fn timestamp(self) -> Option<Timestamp> {
        let nanos = self.nanos.min(NANOS_PER_SECOND - 1);
        Timestamp::new(self.seconds, nanos as i32).ok()
    }

    /// The offset the date is shown at.
    pub const fn offset(self) -> Offset {
        self.offset
    }

    /// The wall-clock fields at the offset.
    pub fn parts(self) -> DateParts {
        let local = self.seconds + i64::from(self.offset.seconds());
        let days = local.div_euclid(SECONDS_PER_DAY);
        let of_day = local.rem_euclid(SECONDS_PER_DAY);
        let (year, month, day) = civil_from_days(days);
        DateParts {
            year,
            month,
            day,
            hour: (of_day / 3600) as i8,
            minute: (of_day / 60 % 60) as i8,
            second: (of_day % 60) as i8,
            nanosecond: self.nanos as i32,
            weekday: (days + 4).rem_euclid(7) as i8,
        }
    }

    /// The year at the offset.
    pub fn year(self) -> i32 {
        self.parts().year
    }

    /// The wall-clock date and time at the offset as a jiff civil date-time, or `None` for years
    /// outside -9999 to 9999. A leap second becomes its minute's last nanosecond.
    pub fn to_civil(self) -> Option<civil::DateTime> {
        let parts = self.parts();
        civil::DateTime::new(
            i16::try_from(parts.year).ok()?,
            parts.month,
            parts.day,
            parts.hour,
            parts.minute,
            parts.second,
            parts.nanosecond.min(NANOS_PER_SECOND as i32 - 1),
        )
        .ok()
    }

    /// A jiff zoned date-time in a fixed time zone at this offset, or `None` where
    /// [`timestamp`](Self::timestamp) is.
    pub fn to_zoned(self) -> Option<ZonedDateTime> {
        Some(
            self.timestamp()?
                .to_zoned_datetime(TimeZone::fixed(self.offset)),
        )
    }

    /// The same instant, shown at `offset`.
    pub const fn with_offset(self, offset: Offset) -> Self {
        Self {
            seconds: self.seconds,
            nanos: self.nanos,
            offset,
        }
    }

    /// The same instant, shown at `time_zone`'s offset for that instant. Past the years jiff's
    /// timestamps reach, the offset is read whole 400-year cycles closer, where the calendar
    /// and a zone's yearly rules repeat.
    pub fn to_time_zone(self, time_zone: &TimeZone) -> Self {
        const CYCLE: i64 = 146_097 * SECONDS_PER_DAY;
        let (min, max) = (Timestamp::MIN.as_second(), Timestamp::MAX.as_second());
        let mut seconds = self.seconds;
        if seconds > max {
            seconds -= ((seconds - max) / CYCLE + 1) * CYCLE;
        } else if seconds < min {
            seconds += ((min - seconds) / CYCLE + 1) * CYCLE;
        }
        let offset = Timestamp::from_second(seconds)
            .map(|timestamp| time_zone.to_offset(timestamp))
            .unwrap_or(Offset::UTC);
        self.with_offset(offset)
    }

    /// The same instant, shown in the system time zone.
    pub fn to_local(self) -> Self {
        self.to_time_zone(&TimeZone::system())
    }

    /// Whole seconds since the Unix epoch, rounded down, as chrono's `timestamp()` gave them. A
    /// leap second counts as its minute's second 59.
    pub const fn unix_seconds(self) -> i64 {
        self.seconds
    }

    /// The nanoseconds after [`unix_seconds`](Self::unix_seconds), as chrono's
    /// `timestamp_subsec_nanos()` gave them: 1,000,000,000 or more for a leap second.
    pub const fn subsec_nanos(self) -> u32 {
        self.nanos
    }

    /// Nanoseconds since the Unix epoch.
    pub fn unix_nanos(self) -> i128 {
        i128::from(self.seconds) * i128::from(NANOS_PER_SECOND) + i128::from(self.nanos)
    }

    /// The instant as a `SystemTime`, or `None` when the platform can't hold it.
    pub fn to_system_time(self) -> Option<SystemTime> {
        SystemTime::try_from(self.timestamp()?).ok()
    }

    /// The date `nanos` nanoseconds later, at the same offset, or `None` past year 262142.
    ///
    /// As with chrono, a leap second stays one while the result is still inside it; otherwise
    /// its extra second is dropped, so `23:59:60` plus one second is `00:00:00`.
    pub fn checked_add_nanos(self, nanos: i64) -> Option<Self> {
        self.checked_add(i128::from(nanos))
    }

    /// The date `seconds` seconds later, at the same offset, or `None` outside years -262143 to
    /// 262142. A leap second is left as [`checked_add_nanos`](Self::checked_add_nanos) leaves it.
    pub fn checked_add_seconds(self, seconds: i64) -> Option<Self> {
        self.checked_add(i128::from(seconds) * i128::from(NANOS_PER_SECOND))
    }

    fn checked_add(self, nanos: i128) -> Option<Self> {
        let mut seconds = i128::from(self.seconds);
        let mut frac = i128::from(self.nanos);
        let second = i128::from(NANOS_PER_SECOND);
        // chrono's `NaiveTime::overflowing_add_signed`: whole seconds round toward zero and the
        // fraction keeps the sign.
        let whole = nanos / second;
        let part = nanos % second;
        if frac >= second {
            if whole > 0 || (part > 0 && frac >= 2 * second - part) {
                // Leaving forward: the leap second ends at the next minute.
                frac -= second;
            } else if whole < 0 {
                // Leaving backward: the leap second counts from the next minute.
                frac -= second;
                seconds += 1;
            } else {
                // Still inside the leap second, or moved back into second 59.
                return Some(Self {
                    nanos: (frac + part) as u32,
                    ..self
                });
            }
        }
        Self::from_unix_nanos(seconds * second + frac + nanos, self.offset)
    }

    /// The date `nanos` nanoseconds earlier, at the same offset, or `None` before year -262143.
    pub fn checked_sub_nanos(self, nanos: i64) -> Option<Self> {
        self.checked_add_nanos(nanos.checked_neg()?)
    }

    /// Nanoseconds from `earlier` to `self`, negative when `self` comes first. As with chrono, a
    /// leap second between the two adds its second only once.
    pub fn nanos_since(self, earlier: Self) -> i128 {
        // chrono subtracts the UTC days and the seconds within the day separately, and counts a
        // leap second by comparing the seconds within the day.
        let split = |date: Self| {
            (
                i128::from(date.seconds.div_euclid(SECONDS_PER_DAY)),
                i128::from(date.seconds.rem_euclid(SECONDS_PER_DAY)),
                date.nanos >= NANOS_PER_SECOND,
            )
        };
        let (days, of_day, leap) = split(self);
        let (earlier_days, earlier_of_day, earlier_leap) = split(earlier);
        let adjust = match of_day.cmp(&earlier_of_day) {
            Ordering::Greater => i128::from(earlier_leap),
            Ordering::Less => -i128::from(leap),
            Ordering::Equal => 0,
        };
        let seconds =
            (days - earlier_days) * i128::from(SECONDS_PER_DAY) + of_day - earlier_of_day + adjust;
        seconds * i128::from(NANOS_PER_SECOND) + i128::from(self.nanos) - i128::from(earlier.nanos)
    }

    /// RFC 3339 with the shortest of 0, 3, 6 or 9 fraction digits and `+00:00` for UTC,
    /// for example `2001-02-03T04:05:06.100+05:30`.
    pub fn to_rfc3339(self) -> String {
        self.to_rfc3339_opts(SecondsFormat::AutoSi, false)
    }

    /// RFC 3339 with the fraction digits `seconds` asks for, and `Z` for UTC when `use_z`.
    pub fn to_rfc3339_opts(self, seconds: SecondsFormat, use_z: bool) -> String {
        let mut out = String::with_capacity(35);
        let parts = self.parts();
        write_date(&mut out, parts, '-');
        out.push('T');
        write_time(&mut out, parts);
        write_fraction(&mut out, parts.nanosecond, seconds, ".");
        if use_z && self.offset.seconds() == 0 {
            out.push('Z');
        } else {
            write_offset(&mut out, self.offset, OffsetStyle::Minutes { colon: true });
        }
        out
    }

    /// RFC 2822, for example `Sat, 3 Feb 2001 04:05:06 +0530`. Years outside 0 to 9999 keep
    /// their sign and digits, which RFC 2822 can't express; callers show those dates as RFC 3339
    /// instead.
    pub fn to_rfc2822(self) -> String {
        let parts = self.parts();
        let mut out = String::with_capacity(31);
        out.push_str(format::SHORT_WEEKDAYS[parts.weekday as usize]);
        out.push_str(", ");
        let day = parts.day as u32;
        push_digits(&mut out, day, if day < 10 { 1 } else { 2 });
        out.push(' ');
        out.push_str(format::SHORT_MONTHS[parts.month as usize - 1]);
        out.push(' ');
        write_year(&mut out, parts.year);
        out.push(' ');
        write_time(&mut out, parts);
        out.push(' ');
        write_offset(&mut out, self.offset, OffsetStyle::Minutes { colon: false });
        out
    }

    /// Format with a `strftime` string, using `locale` for names, `%c %x %X %r` and the
    /// decimal point. See [`format`](self::format) for the specifiers.
    pub fn format(
        self,
        format: &str,
        locale: pure_rust_locales::Locale,
    ) -> Result<String, DateFormatError> {
        format::format(self, format, locale)
    }

    /// Parse strict RFC 3339 the way nushell's date literals always have: `T`, `t` or a space
    /// between date and time, any number of fraction digits, `Z`, `z` or `±HH:MM`, and `:60`.
    pub fn parse_rfc3339(text: &str) -> Result<Self, DateParseError> {
        parse::parse_rfc3339(text)
    }
}

/// The Unix epoch at UTC, as chrono's default was.
impl Default for DateTime {
    fn default() -> Self {
        Self::UNIX_EPOCH
    }
}

impl PartialEq for DateTime {
    fn eq(&self, other: &Self) -> bool {
        (self.seconds, self.nanos) == (other.seconds, other.nanos)
    }
}

impl Eq for DateTime {}

impl PartialOrd for DateTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DateTime {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.seconds, self.nanos).cmp(&(other.seconds, other.nanos))
    }
}

impl Hash for DateTime {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.seconds, self.nanos).hash(state);
    }
}

/// `2001-02-03 04:05:06.100 +05:30`; an offset with seconds keeps them (`+05:30:15`).
impl fmt::Display for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts = self.parts();
        let mut out = String::with_capacity(36);
        write_date(&mut out, parts, '-');
        out.push(' ');
        write_time(&mut out, parts);
        write_fraction(&mut out, parts.nanosecond, SecondsFormat::AutoSi, ".");
        out.push(' ');
        write_offset(&mut out, self.offset, OffsetStyle::Full);
        f.write_str(&out)
    }
}

/// `2001-02-03T04:05:06.100+05:30`; an offset with seconds keeps them (`+05:30:15`).
impl fmt::Debug for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts = self.parts();
        let mut out = String::with_capacity(35);
        write_date(&mut out, parts, '-');
        out.push('T');
        write_time(&mut out, parts);
        write_fraction(&mut out, parts.nanosecond, SecondsFormat::AutoSi, ".");
        write_offset(&mut out, self.offset, OffsetStyle::Full);
        f.write_str(&out)
    }
}

/// Reads relaxed RFC 3339: what [`DateTime::parse_rfc3339`] reads, plus unpadded fields, signed
/// years, spaces between parts, `UTC` and offsets without a colon.
impl FromStr for DateTime {
    type Err = DateParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        parse::parse_rfc3339_relaxed(text)
    }
}

/// Written as RFC 3339 with `Z` for UTC and the shortest of 0, 3, 6 or 9 fraction digits,
/// for example `"2001-02-03T04:05:06.100+05:30"`. Plugins have always received this text.
impl Serialize for DateTime {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_rfc3339_opts(SecondsFormat::AutoSi, true))
    }
}

/// Reads the relaxed RFC 3339 that [`FromStr`] reads, so plugins written in any language can
/// send their own RFC 3339 text.
impl<'de> Deserialize<'de> for DateTime {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;

        impl serde::de::Visitor<'_> for Visitor {
            type Value = DateTime;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("an RFC 3339 date-time string")
            }

            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<DateTime, E> {
                value.parse().map_err(E::custom)
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

/// Days from 1970-01-01 to the given date in the proleptic Gregorian calendar (Howard Hinnant's
/// `days_from_civil`).
fn days_from_civil(year: i32, month: i8, day: i8) -> i64 {
    let year = i64::from(year) - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month = i64::from(month);
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The date `days` after 1970-01-01, as year, month and day (Howard Hinnant's
/// `civil_from_days`).
fn civil_from_days(days: i64) -> (i32, i8, i8) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year as i32, month as i8, day as i8)
}

fn days_in_month(year: i32, month: i8) -> i8 {
    match month {
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => 31,
    }
}

/// How [`write_offset`] writes an offset.
#[derive(Clone, Copy)]
enum OffsetStyle {
    /// `+HH:MM`, or `+HH:MM:SS` when the offset has seconds.
    Full,
    /// Rounded to the minute, `+HH:MM` or `+HHMM`.
    Minutes { colon: bool },
    /// `+HH:MM:SS` or `+HHMMSS`, always with seconds.
    Seconds { colon: bool },
    /// Truncated to the hour, `+HH`.
    Hours,
}

fn write_offset(out: &mut String, offset: Offset, style: OffsetStyle) {
    let total = offset.seconds();
    out.push(if total < 0 { '-' } else { '+' });
    let total = total.unsigned_abs();
    let (hours, minutes, seconds, colon) = match style {
        OffsetStyle::Full => (total / 3600, total / 60 % 60, total % 60, true),
        OffsetStyle::Minutes { colon } => {
            let rounded = (total + 30) / 60;
            (rounded / 60, rounded % 60, 0, colon)
        }
        OffsetStyle::Seconds { colon } => (total / 3600, total / 60 % 60, total % 60, colon),
        OffsetStyle::Hours => (total / 3600, 0, 0, false),
    };
    push_digits(out, hours, 2);
    if matches!(style, OffsetStyle::Hours) {
        return;
    }
    if colon {
        out.push(':');
    }
    push_digits(out, minutes, 2);
    let with_seconds = match style {
        OffsetStyle::Full => seconds != 0,
        OffsetStyle::Seconds { .. } => true,
        _ => false,
    };
    if with_seconds {
        if colon {
            out.push(':');
        }
        push_digits(out, seconds, 2);
    }
}

/// Writes the last `width` decimal digits of `value`, zero-padded. Faster than `write!`, which
/// matters for tables and serialization with many dates.
fn push_digits(out: &mut String, mut value: u32, width: usize) {
    let mut digits = [b'0'; 10];
    for digit in digits[..width].iter_mut().rev() {
        *digit = b'0' + (value % 10) as u8;
        value /= 10;
    }
    for &digit in &digits[..width] {
        out.push(char::from(digit));
    }
}

/// A year as chrono wrote it: at least 4 digits, with `-` before negative years and `+` before
/// years past 9999.
fn write_year(out: &mut String, year: i32) {
    if year < 0 {
        out.push('-');
    } else if year > 9999 {
        out.push('+');
    }
    let digits = year.unsigned_abs();
    push_digits(
        out,
        digits,
        if digits > 9999 {
            decimal_width(digits)
        } else {
            4
        },
    );
}

/// The number of decimal digits in `value`.
fn decimal_width(value: u32) -> usize {
    value.checked_ilog10().map_or(1, |log| log as usize + 1)
}

fn write_date(out: &mut String, parts: DateParts, separator: char) {
    write_year(out, parts.year);
    out.push(separator);
    push_digits(out, parts.month as u32, 2);
    out.push(separator);
    push_digits(out, parts.day as u32, 2);
}

/// `HH:MM:SS`, with a leap second as `60`.
fn write_time(out: &mut String, parts: DateParts) {
    push_digits(out, parts.hour as u32, 2);
    out.push(':');
    push_digits(out, parts.minute as u32, 2);
    out.push(':');
    let second = parts.second as u32 + u32::from(parts.is_leap_second());
    push_digits(out, second, 2);
}

/// Writes `nanos` as a fraction after `point`, with the digits `seconds` asks for. A leap
/// second's extra second is left out, since its seconds field already shows `60`.
fn write_fraction(out: &mut String, nanos: i32, seconds: SecondsFormat, point: &str) {
    let nanos = nanos % NANOS_PER_SECOND as i32;
    let digits = match seconds {
        SecondsFormat::Secs => return,
        SecondsFormat::Millis => 3,
        SecondsFormat::Micros => 6,
        SecondsFormat::Nanos => 9,
        SecondsFormat::AutoSi if nanos == 0 => return,
        SecondsFormat::AutoSi if nanos % 1_000_000 == 0 => 3,
        SecondsFormat::AutoSi if nanos % 1_000 == 0 => 6,
        SecondsFormat::AutoSi => 9,
    };
    out.push_str(point);
    let value = nanos.unsigned_abs() / 10_u32.pow(9 - digits as u32);
    push_digits(out, value, digits);
}

#[cfg(test)]
mod tests;
