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
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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

    /// Return value of duration in nanoseconds, if possible.  
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

    // helper for cmp and eq traits: if units are comparable, returns scaled quantities for comparison
    fn compare_to(&self, other: &Self) -> Option<(i64, i64)> {
        let self_us = self.unit.unit_scale();
        let other_us = other.unit.unit_scale();
        if self_us.1 == other_us.1 {
            if self_us.0 <= other_us.0 {
                Some((
                    self.quantity,
                    other.quantity.checked_mul(other_us.0 / self_us.0)?,
                ))
            } else {
                Some((
                    self.quantity.checked_mul(self_us.0 / other_us.0)?,
                    other.quantity,
                ))
            }
        } else {
            None // can't compare days range with months range durations
        }
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
                let ela_ns = start.signed_duration_since(*end).num_nanoseconds()?;
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
                rhs.checked_add_signed(chrono::Duration::nanoseconds(self.quantity))
            }
            _ => {
                let quantity = self.quantity.checked_mul(self.unit.unit_scale().0)?;
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
                        rhs.checked_add_signed(chrono::Duration::nanoseconds(quantity))
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

impl std::cmp::PartialOrd for NuDuration {
    /// Compare 2 [NuDuration].  When both durations are in same time unit range,
    /// result is based on comparison of quantity (scaled to common units).
    /// When durations have incomparable units (e.g one is `days` and the other `months`,
    /// the operation is not allowed, though there are cases where an answer could be provided (1ns < 1month, for sure).
    ///
    /// Note that trait [Ord] is *not* implemented.  
    /// This would require that all instances can be compared, which is not true of [NuDuration]
    #[allow(clippy::manual_map)] // might add more code in the else later...
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if let Some(q) = self.compare_to(other) {
            Some(i64::cmp(&q.0, &q.1))
        } else {
            None // can't compare incompatible units (at all)
                 // future: think about hueristics we could use in *some* remaining cases.
                 // clearly 1 ns < 1 month.  Why not give an explicit answer?
                 // problem is there will remain some error cases (is 29 days < 1 month?).
                 // these few failures at runtime would be very confusing for an operation that user might come to think is infallible.
        }
    }
}

impl std::cmp::PartialEq for NuDuration {
    /// Determine whether 2 durations are 'equal'.
    /// If they are in the same time unit range, equality is based on
    /// comparison of scaled quantities (in common units).
    /// If durations have incomparable units, return false.
    fn eq(&self, other: &Self) -> bool {
        if let Some(q) = self.compare_to(other) {
            q.0 == q.1
        } else {
            false // incomparable units can't be equal
        }
    }
}
impl Eq for NuDuration {}

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
    #[case(NuDuration::new(23, Unit::Millennium), Err(se_cnc("time unit")))]
    #[case(NuDuration::new(0, Unit::Month), Err(se_cnc("time unit")))]
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

    #[rstest]
    #[case(NuDuration{quantity:2, unit: Unit::Day}, NuDuration{quantity:24, unit:Unit::Hour}, true)] // bigger unit on lhs
    #[case(NuDuration{quantity:48, unit: Unit::Hour},   NuDuration{quantity:1, unit:Unit::Day}, true)] // bigger unit on rhs
    #[case(NuDuration{quantity:999_000_000_000, unit: Unit::Nanosecond},NuDuration{quantity:1, unit:Unit::Week}, false)] // units from extreme ends of range
    #[case(NuDuration{quantity:999_000_000_000, unit: Unit::Millennium},NuDuration{quantity:1, unit:Unit::Month}, true)] // units from extreme ends of range
    #[case(NuDuration{quantity:2, unit: Unit::Week}, NuDuration{quantity:1, unit:Unit::Hour}, true)] // sm quan * bigger unti > small quan * smaller unit
    #[case(NuDuration{quantity:999_000_000_000_000, unit: Unit::Nanosecond},NuDuration{quantity:1, unit:Unit::Week}, true)] // small quan * smaller unit > small quan bigger unit FALSE

    fn test_cmp_greater(
        #[case] lhs: NuDuration,
        #[case] rhs: NuDuration,
        #[case] exp_result: bool,
    ) {
        assert_eq!(lhs > rhs, exp_result);
    }

    #[rstest]
    #[case(NuDuration{quantity:2, unit: Unit::Day}, NuDuration{quantity:24, unit:Unit::Month} )] // incompatible 1
    #[case(NuDuration{quantity:2, unit: Unit::Year}, NuDuration{quantity:24, unit:Unit::Second} )] // incompatible 1
                                                                                                   // this test shows that incompatible units have no fixed ordering: they will never compare greater than *or* less than.
                                                                                                   // I was actually expecting the comparison to simply fail (due to returning None from partial_cmp!  But Rust had other ideas.
    fn test_cmp_error(#[case] lhs: NuDuration, #[case] rhs: NuDuration) {
        assert!(
            (!(lhs < rhs)) && !(lhs > rhs) && !(lhs == rhs),
            "Expected no possible comparison between incompatible types"
        );
    }
}
