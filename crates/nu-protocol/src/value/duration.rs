//!todos
//! implement nested units: start_day_in_... and monday, tuesday (as next or prev day of week)
//! implement is_negative, From, other traits to avoid special case code in ../from_value and ../from
use crate::DURATION_UNIT_GROUPS;
use crate::{ShellError, Span, Unit};
use chrono::{DateTime, Datelike, FixedOffset};
use serde::{Deserialize, Serialize};
use std::{cmp::min, fmt};
use thiserror::Error;

// convenient(?) shorthands for standard types used in this module
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub type BaseDT = DateTime<FixedOffset>; // the one and only chrono datetime we support
pub type UnitSize = i64; // handle duration range errors internally

/*
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
*/
/// High fidelity Duration type for Nushell
///
/// For use with [chrono::DateTime<FixedOffset>) date/times.
///
/// Supports extended duration range: (Years, Months, Weeks, Days) (via [calends::RelativeDuration)
/// and (Hours .. NS) (via [chrono::Duration)).
///
/// Can do mixed datetime/duration arithmetic,
/// Provides additional operators to do *truncating* arithmetic on datetimes
/// with desired precision and resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NuDuration {
    pub quantity: UnitSize, // number of time units
    pub unit: Unit,         // but only the duration units
}

impl NuDuration {
    pub fn new(quantity: UnitSize, unit: Unit) -> Self {
        NuDuration { quantity, unit }
    }
    /// shortcut for the many places that create nanosecond durations
    pub fn ns(ns: UnitSize) -> Self {
        NuDuration {
            quantity: ns,
            unit: Unit::Nanosecond,
        }
    }

    /// Return duration in nanoseconds, if possible.  
    ///
    /// Returns ShellError on failure, either due to:
    /// duration is in the month range; or scaling to nanoseconds caused an overflow.
    /// To convert from month to any day range duration, see `duration | into int --base_date `.

    pub fn to_ns_or_err(&self, span: Span) -> std::result::Result<i64, ShellError> {
        if self.unit.unit_scale().1 == Unit::Nanosecond {
            if let Some(ret_val) = i64::checked_mul(self.quantity, self.unit.unit_scale().0) {
                Ok(ret_val)
            } else {
                Err(ShellError::CouldNotConvertDurationNs {
                    reason: "Overflow".into(),
                    span,
                })
            }
        } else {
            Err(ShellError::CouldNotConvertDurationNs {
                reason: "Incompatible time unit".into(),
                span,
            })
        }
    }

    /// Parse ISO8601 duration string in the form: "PnYnMnDTnHnMn.nnnnnnnnS", returns a **list** of durations.
    ///
    /// Standard doesn't have placeholders for milli- micro- or nano-seconds, uses fractional part instead.
    // Might also accept the "extended" form, "Pyyyy-mm-ddThh:mm:ss.fffffffff".

    #[allow(unused_variables)]
    pub fn from_iso8601(p_string: &str) -> Result<Box<[NuDuration]>> {
        todo!("from_iso8601")
    }

    pub fn unit_name(&self) -> String {
        (match self.unit {
            Unit::Nanosecond => ["nanosecond", "nanoseconds"],
            Unit::Microsecond => ["microsecond", "microseconds"],
            Unit::Millisecond => ["millisecond", "milliseconds"],
            Unit::Second => ["second", "seconds"],
            Unit::Minute => ["minute", "minutes"],
            Unit::Hour => ["hour", "hours"],
            Unit::Day => ["day", "days"],
            Unit::Week => ["week", "weeks"],
            Unit::Month => ["month", "months"],
            Unit::Quarter => ["quarter", "quarters"],
            Unit::Year => ["year", "years"],
            Unit::Century => ["century", "centuries"],
            Unit::Millennium => ["millennium", "millennia"],
            _ => ["", ""], //todo: add singular and plural for other Units (if they become non-scaled types)
        })[if self.quantity == 1 { 0 } else { 1 }]
        .into()
    }

    /// add duration to duration
    ///
    /// Only works when both durations are in same "range" (days or months)
    pub fn add(&self, rhs: &NuDuration) -> Option<NuDuration> {
        if self.unit.unit_scale().1 == rhs.unit.unit_scale().1 {
            let unit = min(self.unit, rhs.unit); // smaller duration unit
            let min_unit_scale = unit.unit_scale().0; // scaling for smaller unit
            let quantity = (self
                .quantity // mul by smaller scales to avoid overflow
                .checked_mul(self.unit.unit_scale().0 / min_unit_scale)?)
            .checked_add(
                rhs.quantity
                    .checked_mul(rhs.unit.unit_scale().0 / min_unit_scale)?,
            )?;
            Some(NuDuration { unit, quantity })
        } else {
            None
        }
    }

    /// date difference, returning a duration in user-specified units
    /// The [end] is *not* included in the duration, this is a <units>-**between** calculation.
    pub fn duration_diff(
        start: &BaseDT,             // start of interval
        end: &BaseDT,               // end of interval
        duration_unit: &NuDuration, // desired units of duration (quantity ignored)
    ) -> Option<NuDuration> {
        match duration_unit.unit.unit_scale().1 {
            Unit::Nanosecond => {
                let ela_ns = end.signed_duration_since(*start).num_nanoseconds()?;
                Some(NuDuration {
                    quantity: ela_ns / duration_unit.unit.unit_scale().0,
                    unit: duration_unit.unit,
                })
            }
            Unit::Month => Some(NuDuration {
                quantity: (signed_month_difference(start, end) / duration_unit.unit.unit_scale().0),
                unit: duration_unit.unit,
            }),
            _ => panic!("misconfigured unit_scale"),
        }
    }

    /// add duration to date/time, return date/time (for chaining)
    /// Returns None if overflow in date calculations
    pub fn add_self_to_date(&self, rhs: &BaseDT) -> Option<BaseDT> {
        match self.unit {
            Unit::Month => {
                if self.quantity < 0 {
                    rhs.checked_sub_months(chrono::Months::new(self.quantity.unsigned_abs() as u32))
                } else {
                    rhs.checked_add_months(chrono::Months::new(self.quantity as u32))
                }
            }
            Unit::Nanosecond => {
                rhs.checked_sub_signed(chrono::Duration::nanoseconds(self.quantity))
            }
            _ => {
                let quantity = self.quantity * self.unit.unit_scale().0;
                match self.unit.unit_scale().1 {
                    Unit::Month => {
                        if self.quantity < 0 {
                            rhs.checked_sub_months(chrono::Months::new(
                                quantity.unsigned_abs() as u32
                            ))
                        } else {
                            rhs.checked_add_months(chrono::Months::new(quantity as u32))
                        }
                    }
                    Unit::Nanosecond => {
                        rhs.checked_sub_signed(chrono::Duration::nanoseconds(quantity))
                    }
                    _ => {
                        panic!("unsupported duration range")
                    }
                }
            }
        }
    }
}

impl fmt::Display for NuDuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}_{}", self.quantity, self.unit_name())
    }
}

impl std::ops::Neg for NuDuration {
    type Output = Self;

    fn neg(self) -> Self::Output {
        NuDuration {
            quantity: -self.quantity,
            unit: self.unit,
        }
    }
}

impl TryFrom<&str> for NuDuration {
    type Error = NuDurationError;

    fn try_from(s: &str) -> std::result::Result<Self, Self::Error> {
        let unit_boundary = s
            .char_indices()
            .find_map(|(i, c)| if c.is_alphabetic() { Some(i) } else { None })
            .ok_or(NuDurationError::UnrecognizedUnit)?;

        let numeric = &s[..unit_boundary];
        let units = &s[unit_boundary..];

        if let Some((unit, _name, _convert)) = DURATION_UNIT_GROUPS.iter().find(|x| units == x.1) {
            let num_part = numeric.replace('_', "");
            match num_part.parse::<i64>() {
                Ok(quantity) => Ok(NuDuration {
                    quantity,
                    unit: *unit,
                }),
                Err(_) => Err(NuDurationError::UnrecognizedIntQuantity),
            }
        } else {
            Err(NuDurationError::UnrecognizedUnit)
        }
    }
}

/// difference between 2 date/times, in integer months
/// Doesn't check for overflow, so truly unreasonable start/end values can panic.
///
/// [chrono] doesn't implement a date-difference-in-months, why?  It seems (gulp) straightforward.
/// This, despite the fact that [chrono::Months] and [chrono::NaiveDate] and friends all implement
/// `date_add` and `date_sub`.
pub fn signed_month_difference(start: &BaseDT, end: &BaseDT) -> UnitSize {
    let end_naive = end.date_naive();
    let start_naive = start.date_naive();

    let month_diff = end_naive.month() as UnitSize - start_naive.month() as UnitSize;
    let years_diff = (end_naive.year() - start_naive.year()) as UnitSize;
    if month_diff >= 0 {
        (years_diff * 12) + month_diff
    } else {
        (years_diff - 1) * 12 + (month_diff + 12)
    }
}

/// Potential errors
#[derive(Copy, Clone, Debug, PartialEq, Hash, Error, Serialize, Deserialize)]
pub enum NuDurationError {
    #[error("Invalid RFC 3339 format datetime string")]
    InvalidDateTimeRfcString,
    #[error("Unrecognized time unit")]
    UnrecognizedUnit,
    #[error("Unrecognized int quantity")]
    UnrecognizedIntQuantity,
    #[error("Chrono nanoseconds overflow")]
    NsOverflow,
    #[error("Chrono days/months overflow")]
    DMOverflow,
    #[error("Ambiguous timezone conversion")]
    AmbiguousTzConversion,
    #[error("Incompatible units")]
    IncompatibleUnits,
    #[error("Test failed")]
    TestFailed, // because
}

#[cfg(test)]
mod test {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(Unit::Microsecond, Unit::Nanosecond)]
    #[case(Unit::Millennium, Unit::Day)]
    fn test_unit_compare(#[case] bigger: Unit, #[case] smaller: Unit) {
        assert!(
            (bigger > smaller),
            "expected {:?} to compare bigger than {:?}",
            bigger,
            smaller
        );
    }

    #[rstest]
    #[case("1ns", Ok(NuDuration{quantity: 1, unit: Unit::Nanosecond}))]
    #[case("____ns", Err(NuDurationError::UnrecognizedIntQuantity))]
    #[case("ns", Err(NuDurationError::UnrecognizedIntQuantity))]
    #[case("10234", Err(NuDurationError::UnrecognizedUnit))]
    #[case("__1__ns", Ok(NuDuration{quantity: 1, unit: Unit::Nanosecond}))]
    #[case("1_foons", Err(NuDurationError::UnrecognizedUnit))]
    #[case("6_d", Ok(NuDuration{quantity: 6, unit:Unit::Day}))]
    #[case("6_da", Ok(NuDuration{quantity: 6, unit:Unit::Day}))]
    #[case("6_day", Ok(NuDuration{quantity: 6, unit:Unit::Day}))]
    #[case("6_days", Ok(NuDuration{quantity: 6, unit:Unit::Day}))]
    #[case("6_d", Ok(NuDuration{quantity: 6, unit:Unit::Day}))]
    #[case("6_d", Ok(NuDuration{quantity: 6, unit:Unit::Day}))]
    #[case("9_223_372_036_854_775_807_millennia", Ok(NuDuration{quantity: 9223372036854775807, unit:Unit::Millennium}))]
    #[case("__0__ns", Ok(NuDuration{quantity: 0, unit: Unit::Nanosecond}))]
    #[case("6_d", Ok(NuDuration{quantity: 6, unit:Unit::Day}))]
    #[case("6.02e23_weeks", Err(NuDurationError::UnrecognizedUnit))]
    #[case("6.02e23_foo", Err(NuDurationError::UnrecognizedUnit))]
    fn test_try_from(
        #[case] instr: &str,
        #[case] expected: std::result::Result<NuDuration, NuDurationError>,
    ) -> Result<()> {
        let observed = NuDuration::try_from(instr);
        assert_eq!(expected, observed);

        Ok(())
    }

    #[rstest]
    #[case("2021-10-01T01:02:03Z", "2021-11-30T23:59:59Z", 1)] // start month < end month, same year
    #[case("2021-10-01T01:02:03Z", "2021-10-30T23:59:59Z", 0)] // start = end, same year
    #[case("2021-11-01T01:02:03Z", "2021-10-30T23:59:59Z", -1)] // start > end, same year
    #[case("2021-10-01T01:02:03Z", "2021-10-30T23:59:59Z", 0)] // start < end, but same month, same year
    #[case("2021-12-01T01:02:03Z", "2022-01-30T23:59:59Z", 1)] // start < end, later year
    #[case("2022-01-01T01:02:03Z", "2021-12-30T23:59:59Z", -1)] // start < end, earlier year
    #[case("1492-10-12T01:02:03Z", "4092-12-30T23:59:59Z", 31202)] // big positive
    fn test_signed_month_difference(
        #[case] start: &str,
        #[case] end: &str,
        #[case] exp_diff: UnitSize,
    ) -> Result<()> {
        let start_dt = &BaseDT::parse_from_rfc3339(start).unwrap();
        let end_dt = &BaseDT::parse_from_rfc3339(end).unwrap();

        let obs_diff = signed_month_difference(start_dt, end_dt);

        assert_eq!(exp_diff, obs_diff);

        Ok(())
    }

    #[rstest]
    #[case(
        NuDuration::new(1, Unit::Nanosecond),
        NuDuration::new(2, Unit::Microsecond),
        Some(NuDuration::new(2001, Unit::Nanosecond))
    )] // similar units, positive
    #[case(
        NuDuration::new(-100, Unit::Nanosecond),
        NuDuration::new(2, Unit::Microsecond),
        Some(NuDuration::new(1900, Unit::Nanosecond))
    )] // similar units, negative
    #[case(
        NuDuration::new(-2, Unit::Millisecond),
        NuDuration::new(2, Unit::Microsecond),
        Some(NuDuration::new(-1998, Unit::Microsecond))
    )] // Negative result, and smaller unit chosen
    #[case(
        NuDuration::new(UnitSize::MAX-2, Unit::Nanosecond),
        NuDuration::new(4, Unit::Nanosecond), // but arg can't require any multipication, or panic
        None,
    )] // Result should fail in expected way
    #[case(
        NuDuration::new(UnitSize::MAX-2, Unit::Nanosecond),
        NuDuration::new(2, Unit::Nanosecond), // but arg can't require any multipication, or panic
        Some(NuDuration::new(UnitSize::MAX, Unit::Nanosecond))
    )] // Negative result, and smaller unit chosen
    #[case(
        NuDuration::new(UnitSize::MIN + 2, Unit::Second),
        NuDuration::new(-4 , Unit::Nanosecond),
        None,
    )] // Negative result, and smaller unit chosen

    fn test_duration_add_duration(
        #[case] lhs: NuDuration,
        #[case] rhs: NuDuration,
        #[case] exp: Option<NuDuration>,
    ) {
        let obs = lhs.add(&rhs);
        assert_eq!(exp, obs);
    }

    fn se_cnc(reason_pat: &str) -> ShellError {
        ShellError::CouldNotConvertDurationNs {
            reason: reason_pat.into(),
            span: Span { start: 0, end: 0 },
        }
    }

    #[rstest]
    #[case(NuDuration::new(1, Unit::Nanosecond), Ok(1))]
    #[case(NuDuration::new(0, Unit::Nanosecond), Ok(0))]
    #[case(NuDuration::new(23, Unit::Day), Ok(23 * 24 * 3600 * 1000000000))]
    #[case(
        NuDuration::new(23, Unit::Millennium),
        Err(se_cnc("time unit millennia"))
    )]
    #[case(NuDuration::new(0, Unit::Month), Err(se_cnc("time unit months")))]
    #[case(NuDuration::new(i64::MAX, Unit::Second), Err(se_cnc("Overflow")))]

    fn test_to_int(
        #[case] duration: NuDuration,
        #[case] expected: core::result::Result<i64, ShellError>,
    ) {
        let result = duration.to_ns_or_err(Span { start: 0, end: 0 });
        match (&expected, &result) {
            (Ok(exp_val), Ok(val)) => assert_eq!(exp_val, val),
            (
                Err(ShellError::CouldNotConvertDurationNs {
                    reason: exp_reason, ..
                }),
                Err(ShellError::CouldNotConvertDurationNs {
                    reason: val_reason, ..
                }),
            ) => {
                assert!(
                    val_reason.contains(exp_reason),
                    "error reason: exp {:?}, act: {:?}",
                    exp_reason,
                    val_reason
                );
            }
            _ => panic!("unexpected error {:?}", result),
        }
    }
}
