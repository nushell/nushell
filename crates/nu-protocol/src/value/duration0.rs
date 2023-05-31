#![allow(unused_imports, dead_code)]
use calends::RelativeDuration;
use chrono::{
    DateTime, Duration, FixedOffset, Months, NaiveDate, NaiveDateTime, NaiveTime, SecondsFormat,
};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

use thiserror::Error;

// convenient(?) shorthands for standard types used in this module
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub type BaseDT = DateTime<FixedOffset>; // the one and only chrono datetime we support
pub type UnitSize = i64; // handle duration range errors internally

// In the hopes the compiler gods are paying attention and will optimize this...
// most PC architectures have integer div instruction
// that returns quotient and remainder in one instruction
#[inline]
fn divmod(dividend: UnitSize, divisor: UnitSize) -> (UnitSize, UnitSize) {
    if divisor == 0 {
        (0, 0)
    } else {
        (dividend / divisor, dividend % divisor)
    }
}
#[inline]
fn divmod_i32(dividend: i32, divisor: i32) -> (i64, i64) {
    if divisor == 0 {
        (0, 0)
    } else {
        ((dividend / divisor) as i64, (dividend % divisor) as i64)
    }
}
/// High fidelity Duration type for Nushell
///
/// For use with [chrono::DateTime<FixedOffset>] date/times.
///
/// Supports extended duration range: (Years, Months, Weeks, Days) (via [calends::RelativeDuration)
/// and (Hours .. NS) (via [chrono::Duration]).
///
/// Can do mixed datetime/duration arithmetic,
/// Provides additional operators to do *truncating* arithmetic on datetimes
/// with desired precision and resolution.
///
/// [NuDuration] is actually a sort of Interval, because it retains a base date.
/// This allows [NuDuration] + [NuDuration] to be well defined, for example.
/// 
/// Open design questions: 
/// 1. Need to retain a base date?  Or could user provide date with [NuDuration::normalize] if needed?
/// 2. The embedded durations naturally support multiple time unit "places", can represent P3M-1D, for example.
///    Is that complexity necessary to expose, or should we ensure each duration represents just one time unit?
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct NuDuration {
    base: BaseDT,          // base date
    msb: RelativeDuration, // days, weeks, months of duration
    lsb: Duration,         // ns .. hours of duration
}

impl NuDuration {
    pub fn from_string(_interval: &str) -> Result<NuDuration> {
        todo!();
    }

    pub fn from_struct(interval: NuDurationStruct) -> Result<NuDuration> {
        Ok(NuDuration {
            base: interval.base,
            lsb: Duration::hours(interval.hours)
                + Duration::minutes(interval.minutes)
                + Duration::seconds(interval.seconds)
                + Duration::nanoseconds(interval.fraction),
            msb: RelativeDuration::from_mwd(
                interval.months.try_into()?,
                interval.weeks.try_into()?,
                interval.days.try_into()?,
            ),
        })
    }

    pub fn from_b_d_ns(
        base: BaseDT,
        days: i32,             // (signed) offset in days
        nanoseconds: UnitSize, // additional offset in nanoseconds (< 1 month)
    ) -> NuDuration {
        NuDuration {
            base,
            lsb: Duration::nanoseconds(nanoseconds),
            msb: RelativeDuration::days(days),
        }
    }

    /// Normalize internal representation, ensuring
    /// each # hours < 24, # months < 12, etc. for each time unit.
    ///
    /// This is where dependency on base date/time comes from - to carry over from days to months.
    /// But we only use date portion of base for normalization -- we would not want
    /// a base datetime near midnight + duration of 1 hour to normalize to 1 day, -23 hours.
    pub fn normalize(&self) -> Result<Self> {
        let normalize_base = NaiveDateTime::new(
            self.base.date_naive(),
            NaiveTime::from_hms_opt(0, 0, 0).expect("constant time failed"),
        );
        let end_dt =
            NaiveDateTime::new(normalize_base.date() + self.msb, normalize_base.time()) + self.lsb; // add Duration last so days can carry over to msb
                                                                                                    //println!("end_dt: {end_dt:#?}");
        Ok(NuDuration {
            base: self.base,
            msb: RelativeDuration::from_duration_between(normalize_base.date(), end_dt.date()),
            lsb: end_dt.time() - normalize_base.time(),
        })
    }

    /// Returns struct with current, unnormalized fields
    pub fn as_struct(&self) -> Result<NuDurationStruct> {
        // reduce sub-day units first

        let (seconds, fraction) = divmod(self.ns_from_lsb()?, 1_000_000_000);
        let (minutes, seconds) = divmod(seconds, 60);
        let (hours, minutes) = divmod(minutes, 60);
        let (days, hours) = divmod(hours, 24);

        // assume normalize rationalized msb, so can use days and weeks directly.
        // also assume it maximized months, so we can scale that for months and years.

        let (years, months) = divmod(self.msb.num_months() as UnitSize, 12);

        Ok(NuDurationStruct::new(
            self.base,
            years,
            months,
            self.msb.num_weeks() as UnitSize,
            self.msb.num_days() as UnitSize  + days,
            hours,
            minutes,
            seconds,
            fraction,
        ))
    }

    /// field getters - inverse of [from_b_d_ns].
    #[inline]
    pub fn get_base(&self) -> BaseDT {
        self.base
    }
    #[inline]
    pub fn get_days(&self) -> UnitSize {
        self.days_from_msb()
    }
    #[inline]
    pub fn get_nanoseconds(&self) -> UnitSize {
        self.ns_from_lsb().unwrap_or(0)
    }

    // number of days in msb of duration (from months through days)
    #[inline]
    pub fn days_from_msb(&self) -> UnitSize {
        (((self.msb.num_months() * 12) + self.msb.num_weeks() * 4) + self.msb.num_days())
            as UnitSize
    }

    // number of nanoseconds in lsb of duration (from hours on down)
    #[inline]
    pub fn ns_from_lsb(&self) -> Result<UnitSize> {
        self.lsb.num_nanoseconds().ok_or(NuDurationError::NsOverflow.into())
    }

    /// Add date and duration
    pub fn add_datetime(&self, dt: BaseDT) -> Result<BaseDT> {
        (NaiveDateTime::new(dt.date_naive() + self.msb, dt.time()) + self.lsb)
            .and_local_timezone(*dt.offset())
            .single()
            .ok_or(NuDurationError::AmbiguousTzConversion.into())
    }

    /// subtract 2 datetimes yielding duration
    /// a kind of constructor, not arithmetic op, arg order is start, end.
    pub fn from_datetime_diff(start: BaseDT, end: BaseDT) -> Result<NuDuration> {
        let ret_val = NuDuration {
            base: start,
            msb: RelativeDuration::from_duration_between(start.date_naive(), end.date_naive()),
            lsb: (end.time() - start.time()),
        };

        Ok(ret_val)
    }

    /// Add duration + duration
    pub fn add_duration(&self, rhs: NuDuration) -> Result<NuDuration> {
        let ret_val = NuDuration {
            base: self.base,
            msb: self.msb + rhs.msb,
            lsb: self.lsb + rhs.lsb,
        };

        Ok(ret_val)
    }

    /// Subtract 2 durations
    pub fn sub_duration(&self, rhs: NuDuration) -> Result<NuDuration> {
        let ret_val = NuDuration {
            base: self.base,
            msb: self.msb - rhs.msb,
            lsb: self.lsb - rhs.lsb,
        };

        Ok(ret_val)
    }
}

/*
impl Display for NuDuration {
    /// Format inverse of [NuDuration::from_string]
    todo!();
}
*/

/// Serde representation of NuDuration
#[derive(Debug, Default, PartialEq)]
pub struct NuDurationStruct {
    pub base: BaseDT,    // base date/time
    pub years: UnitSize, // (signed) offsets
    pub months: UnitSize,
    pub weeks: UnitSize,
    pub days: UnitSize,
    pub hours: UnitSize,
    pub minutes: UnitSize,
    pub seconds: UnitSize,
    pub fraction: UnitSize, // fractional number of nanoseconds (< 1 sec)
}
impl NuDurationStruct {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        base: BaseDT,    // base date/time
        years: UnitSize, // (signed) offsets
        months: UnitSize,
        weeks: UnitSize,
        days: UnitSize,
        hours: UnitSize,
        minutes: UnitSize,
        seconds: UnitSize,
        fraction: UnitSize, // fractional number of nanoseconds (< 1 sec)
    ) -> Self {
        NuDurationStruct {
            base,
            years,
            months,
            weeks,
            days,
            hours,
            minutes,
            seconds,
            fraction,
        }
    }
}

#[derive(Copy, Clone, Debug, Error, Serialize, Deserialize)]
pub enum NuDurationError {
    #[error("Invalid RFC 3339 format datetime string")]
    InvalidDateTimeRfcString,
    #[error("Chrono nanoseconds overflow")]
    NsOverflow,
    #[error("Chrono days/months overflow")]
    DMOverflow,
    #[error("Ambiguous timezone conversion")]
    AmbiguousTzConversion,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::format::Fixed;
    use rstest::rstest;

    fn sample_dates(n: usize) -> BaseDT {
        let chosen = [
            "2021-10-09T23:59:59.999999999+05:00", // local and nearly midnight
            "2021-10-09T00:01:02.012345678Z",      // utc early in day
            "1492-10-01T23:22:21Z",                 // long past date
            "9999-10-01T23:22:21Z",                 // far future date
        ][n];
        BaseDT::parse_from_rfc3339( chosen).expect("couldn't parse constant date")
    }

    #[rstest]
    #[case(NuDuration::from_b_d_ns(sample_dates(0), 0, 25*3600*1_000_000_000),
    NuDurationStruct::new(sample_dates(0),0,0,0, 1,1,0,0,0))]
    #[case(NuDuration::from_b_d_ns(sample_dates(0), 5, 25*3600*1_000_000_000),
    NuDurationStruct::new(sample_dates(0),0,0,0, 6,1,0,0,0))]

    fn test_normalize(#[case] old: NuDuration, #[case] exp: NuDurationStruct) -> Result<()> {
        let norm_dur = old.normalize()?;
        let nd_struct = norm_dur.as_struct()?;
        assert_eq!(exp, nd_struct, "normalized struct as predicted");
        let old_end = old.add_datetime(old.base)?;
        let norm_dur_end = norm_dur.add_datetime(old.base)?;
        
        assert_eq!(
            old_end.to_rfc3339_opts(SecondsFormat::Nanos, true),
            norm_dur_end.to_rfc3339_opts(SecondsFormat::Nanos, true),
            "normalized end date same as original"
        );
        Ok(())
    }

    #[rstest]
    #[case(NuDuration::from_b_d_ns( sample_dates(2), 33,  24 * 3600 * 1_000_000_000 - 1),
        NuDurationStruct::new(sample_dates(2), 0,0,0,33,23, 59,59,999_999_999))]
    #[case(NuDuration::from_b_d_ns( sample_dates(2), 33,  144 * 24 * 3600 * 1_000_000_000 - 1),
        NuDurationStruct::new(sample_dates(2), 0,0,0,177,23, 59,59,999_999_999))]

    fn initialize_and_query(
        #[case] dur: NuDuration,
        #[case] exp_struct: NuDurationStruct,
    ) -> Result<()> {
        
        let nds = dur.as_struct()?;

        assert_eq!(exp_struct, nds);

        Ok(())
    }
    /*
    #[rstest]
    fn add_duration_date(
        #[case] dur: &str,
        #[case] date: &str,
        #[case] exp_date: &str,
    ) -> Result<()> {}
    */
}
