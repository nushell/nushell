#![allow(unused_imports, dead_code)]
use calends::RelativeDuration;
use chrono::{DateTime, Duration, FixedOffset};
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
/// Supports extended duration range: (Years, Months) (via [calends::RelativeDuration)
/// and (Weeks, Days, Hours .. NS) (via [chrono::Duration]).
///
/// Can do mixed datetime/duration arithmetic,
/// Provides additional operators to do *truncating* arithmetic on datetimes
/// with desired precision and resolution.
///
/// [NuDuration] is actually a sort of Interval, because it retains a base date.
/// This allows [NuDuration] + [NuDuration] to be well defined, for example.
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

    /// Normalize internal representation, ensuring lsb < 1 day.
    /// This is where dependency on base date/time comes from - to carry over from days to months.
    pub fn normalize(&mut self) -> Result<()> {
        let overflow_days = self.lsb.num_days();
        if overflow_days != 0 {
            // carry days out of lsb to msb
            self.lsb = self.lsb - Duration::days(overflow_days);
            self.msb = self.msb + self.msb.with_days(overflow_days.try_into()?);

            // (hopefully) normalize month, week, day in msb
            let end_date = self.base.date_naive() + self.msb;
            self.msb = RelativeDuration::from_duration_between(self.base.date_naive(), end_date);
        };
        Ok(())
    }

    /// Returns struct with "normalized" fields
    pub fn as_struct(&mut self) -> Result<NuDurationStruct> {
        self.normalize()?;

        // reduce sub-day units first

        let (seconds, fraction) = divmod(self.ns_from_lsb()?, 1_000_000_000);
        let (minutes, seconds) = divmod(seconds, 60);
        let (hours, minutes) = divmod(minutes, 60);
        let (days, hours) = divmod(hours, 24);

        assert_eq!(days, 0, "excess overflow days");

        // assume normalize rationalized msb, so can use days and weeks directly.
        // also assume it maximized months, so we can scale that for months and years.

        let (months, years) = divmod(self.msb.num_months() as UnitSize, 12);

        Ok(NuDurationStruct::new(
            self.base,
            years,
            months,
            self.msb.num_weeks() as UnitSize,
            self.msb.num_days() as UnitSize,
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
    fn days_from_msb(&self) -> UnitSize {
        (((self.msb.num_months() * 12) + self.msb.num_weeks() * 4) + self.msb.num_days())
            as UnitSize
    }

    // number of nanoseconds in lsb of duration (from hours on down)
    #[inline]
    fn ns_from_lsb(&self) -> Result<UnitSize> {
        if let Some(ns) = self.lsb.num_nanoseconds() {
            Ok(ns)
        } else {
            Err(NuDurationError::NsOverflow())?
        }
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
    InvalidDateTime(),
    #[error("Chrono nanoseconds overflow")]
    NsOverflow(),
}

pub fn new_err(e: NuDurationError) -> Box<NuDurationError> {
    Box::new(e)
}
#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("2020-10-01T23:22:21Z", 33,  24 * 3600 * 1_000_000_000 - 1,
(0,0,0,33,23, 59,59,999_999_999))]
    fn initialize_and_query(
        #[case] basestr: &str,
        #[case] days: i32,
        #[case] ns: i64,
        #[case] ev: (
            UnitSize,
            UnitSize,
            UnitSize,
            UnitSize,
            UnitSize,
            UnitSize,
            UnitSize,
            UnitSize,
        ),
    ) -> Result<()> {
        let base = DateTime::parse_from_rfc3339(basestr).expect("should work");
        let exp_struct =
            NuDurationStruct::new(base, ev.0, ev.1, ev.2, ev.3, ev.4, ev.5, ev.6, ev.7);

        let mut nd = NuDuration::from_b_d_ns(base, days, ns);
        let nds = nd.as_struct()?;

        assert_eq!(exp_struct, nds);
        Ok(())
    }
}
