//!todos
//! implement nested units: start_day_in_... and monday, tuesday (as next or prev day of week)
//! implement is_negative, From, other traits to avoid special case code in ../from_value and ../from
use crate::DURATION_UNIT_GROUPS;
use crate::{ShellError, Span, Unit};
use chrono::{DateTime, Datelike, FixedOffset};
use serde::{Deserialize, Serialize};
use std::{cmp::min, cmp::Ordering, fmt};
use thiserror::Error;

// convenient(?) shorthands for standard types used in this module
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub type BaseDT = DateTime<FixedOffset>; // the one and only chrono datetime we support
pub type UnitSize = i64; // size of quantity int

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
fn divmod_i32(dividend: i32, divisor: i32) -> (UnitSize, UnitSize) {
    if divisor == 0 {
        (0, 0)
    } else {
        ((dividend / divisor) as UnitSize, (dividend % divisor) as UnitSize)
    }
}
*/

/// High fidelity Duration type for Nushell
///
/// For use with [chrono::DateTime<FixedOffset>) date/times.
///
/// Supports extended duration range: (Years, Months, Weeks, Days) as well as
/// (Hours .. NS) (via [chrono::Duration)).
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

    pub fn to_ns_or_err(&self, span: Span) -> std::result::Result<UnitSize, ShellError> {
        if self.unit.unit_scale().1 == Unit::Nanosecond {
            if let Some(ret_val) = UnitSize::checked_mul(self.quantity, self.unit.unit_scale().0) {
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
    fn compare_to(&self, other: &Self) -> Option<(UnitSize, UnitSize)> {
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

    // provide data for "incomparable units" heuristics
    // the key insight is that duration < 28 days must be less than 1 month
    // and that duration > 31 days must be greater than 1 month, no matter how many days in whichever month.
    // We (somewhat arbitrarily) implement that duration == 28 days (to the nanosecond) is *not* less than 1 month,
    // but that duration == 31 days (to the nanosecond) is *greater than* 1 month.
    // if we wanted to refine the comparable range even more, we could observe that a duration of less than 365 days must be less than 1 year,
    // regardless of leap year, but that doesn't seem worth 2 checked multiplies.
    fn compare_days_months(&self, other: &Self) -> Option<DaysMonthsResult> {
        let self_us = self.unit.unit_scale();
        let other_us = other.unit.unit_scale();
        debug_assert!(
            (self_us.1 == Unit::Nanosecond && other_us.1 == Unit::Month)
                || (self_us.1 == Unit::Month && other_us.1 == Unit::Nanosecond)
        );

        let days_us = Unit::Day.unit_scale();
        let months_us = Unit::Month.unit_scale();
        if self_us.1 == Unit::Nanosecond {
            Some(DaysMonthsResult {
                lhs_is_days: true,
                days: UnitSize::checked_mul(self.quantity, self_us.0)? / days_us.0,
                months: UnitSize::checked_mul(other.quantity, other_us.0)? / months_us.0,
            })
        } else if self_us.1 == Unit::Month {
            Some(DaysMonthsResult {
                lhs_is_days: false,
                days: UnitSize::checked_mul(other.quantity, other_us.0)? / days_us.0,
                months: UnitSize::checked_mul(self.quantity, self_us.0)? / months_us.0,
            })
        } else {
            None
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

// helper struct for compare_months_days() (q.v.)
struct DaysMonthsResult {
    lhs_is_days: bool, // true if lhs was the "days" range input; else rhs was.
    days: UnitSize,    // number of days in the "days" range input (whichever it was)
    months: UnitSize,  // number of months in the "months" range input (whichever that was)
}
impl fmt::Display for NuDuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}_{}", self.quantity, self.unit_name())
    }
}

impl std::cmp::PartialOrd for NuDuration {
    /// Compare 2 [NuDuration], attempt to provide a concrete ordering.
    /// When both durations are in same time unit range, can always provide a concrete result,
    /// both quantities are scaled to common units.
    /// When durations have incomparable units (e.g one is `days` and the other `months`),
    /// heuristics can provide a concrete result for some inputs (e.g, 28_days < 1_month).
    ///
    /// Note that trait [Ord] is *not* implemented.  
    /// This would require that all instances can be compared, which is not true of [NuDuration]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if let Some(q) = self.compare_to(other) {
            Some(UnitSize::cmp(&q.0, &q.1))
        } else {
            if let Some(cdm) = self.compare_days_months(other) {
                if cdm.days < cdm.months * 28 {
                    if cdm.lhs_is_days {
                        Some(Ordering::Less)
                    } else {
                        Some(Ordering::Greater)
                    }
                } else if cdm.days >= cdm.months * 31 {
                    if cdm.lhs_is_days {
                        Some(Ordering::Greater)
                    } else {
                        Some(Ordering::Less)
                    }
                } else {
                    None
                }
            } else {
                None // can't compare incompatible units (at all)
            }
        }
    }
}

impl std::cmp::PartialEq for NuDuration {
    /// Determine whether 2 durations are 'equal'.
    /// They could be equal if:
    /// * both durations are 0 (regardless of units),
    /// * or if both durations have *same* range of time unit, based on equality of scaled quantities,
    /// Otherwise, durations have incomparable units, can't be equal, return false.
    fn eq(&self, other: &Self) -> bool {
        if self.quantity == 0 && other.quantity == 0 {
            true
        } else if let Some(q) = self.compare_to(other) {
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
            match num_part.parse::<UnitSize>() {
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
    #[case(NuDuration::new(UnitSize::MAX, Unit::Second), Err(se_cnc("Overflow")))]

    fn test_to_int(
        #[case] duration: NuDuration,
        #[case] expected: core::result::Result<UnitSize, ShellError>,
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
    #[case(30, Unit::Day, 30*24, Unit::Hour, true)] // equal is equal direct or reversed
    #[case(120, Unit::Month, 10, Unit::Year, true)] // equal is equal direct or reversed, months range
    #[case(2, Unit::Nanosecond, 2, Unit::Week, false)] // comparable and not equal
    #[case(100, Unit::Month, 1000, Unit::Year, false)] // comparable and not equal, months range
    fn test_eq(
        #[case] lhs_quan: UnitSize,
        #[case] lhs_unit: Unit,
        #[case] rhs_quan: UnitSize,
        #[case] rhs_unit: Unit,
        #[case] exp_eq: bool,
    ) {
        let lhs = NuDuration {
            quantity: lhs_quan,
            unit: lhs_unit,
        };
        let rhs = NuDuration {
            quantity: rhs_quan,
            unit: rhs_unit,
        };

        assert_eq!(exp_eq, lhs.eq(&rhs), "eq: expected matches observed");
        assert_eq!(
            exp_eq,
            rhs.eq(&lhs),
            "reversed eq: expected matches observed"
        );
    }

    #[rstest]
    #[case(NuDuration{quantity:29, unit: Unit::Day}, NuDuration{quantity:1, unit:Unit::Month} )] // incompatible 1
                                                                                                 // this test shows that incompatible units have no fixed ordering: they will never compare greater than *or* less than.
                                                                                                 // I was actually expecting the comparison to simply fail (due to returning None from partial_cmp!  But Rust had other ideas.
    fn test_cmp_error(#[case] lhs: NuDuration, #[case] rhs: NuDuration) {
        assert!(!(lhs < rhs), "incomparable not less than");
        assert!(!(lhs > rhs), "incomparable not greater than");
        assert!(!(lhs == rhs), "incomparable not equal");
    }

    #[rstest]
    // cmp equal
    #[case(30, Unit::Day, 30*24, Unit::Hour, Some(Ordering::Equal) )] // equal is equal direct or reversed
    #[case(120, Unit::Month, 10, Unit::Year, Some(Ordering::Equal))] // equal is equal direct or reversed, months range
    // cmp less than
    #[case(2, Unit::Nanosecond, 2, Unit::Week, Some(Ordering::Less))] // comparable and not equal
    #[case(100, Unit::Month, 1000, Unit::Year, Some(Ordering::Less))] // comparable and not equal, months range
    #[case(999_000_000_000, Unit::Nanosecond, 1, Unit::Week, Some(Ordering::Less))] // units from extreme ends of range
    // cmp greater than
    #[case(2, Unit::Day, 24, Unit::Hour, Some(Ordering::Greater))] // bigger unit on lhs
    #[case(48, Unit::Hour, 1, Unit::Day, Some(Ordering::Greater))] // bigger unit on rhs
    #[case(
        999_000_000_000,
        Unit::Millennium,
        1,
        Unit::Month,
        Some(Ordering::Greater)
    )] // units from extreme ends of range
    #[case(2, Unit::Week, 1, Unit::Hour, Some(Ordering::Greater))] // sm quan * bigger unti > small quan * smaller unit
    // incomparable, resolved via heuristics
    #[case(2, Unit::Nanosecond, 1, Unit::Month, Some(Ordering::Less))] // incomparable, but obvious winner
    #[case(1, Unit::Month, 2, Unit::Nanosecond, Some(Ordering::Greater))] // incomparable, but winner switches sides
    #[case(28, Unit::Day, 1, Unit::Month, None)] // incomparable, edge case where we can compare
    #[case(31, Unit::Day, 1, Unit::Month, Some(Ordering::Greater))] // incomparable, edge case where we can compare
    // cmp probe edges of heuristics
    #[case(28*24*3600*1_000_000_000 - 1, Unit::Nanosecond, 1, Unit::Month, Some(Ordering::Less))] // incomparable, but just before the grey area
    #[case(28*24*3600*1_000_000_000, Unit::Nanosecond, 1, Unit::Month, None)] // incomparable, in grey area
    #[case(28*24*3600*1_000_000_000 + 1, Unit::Nanosecond, 1, Unit::Month, None)] // incomparable, in grey area
    #[case(31*24*3600*1_000_000_000 - 1, Unit::Nanosecond, 1, Unit::Month, None)] // incomparable, in grey area
    #[case(31*24*3600*1_000_000_000, Unit::Nanosecond, 1, Unit::Month, Some(Ordering::Greater))] // incomparable, hard case, day + 0ns is first instant beyond grey area
    #[case(31*24*3600*1_000_000_000 + 1, Unit::Nanosecond, 1, Unit::Month, Some(Ordering::Greater))] // incomparable, but just beyond the grey area

    fn test_partial_cmp(
        #[case] lhs_quan: UnitSize,
        #[case] lhs_unit: Unit,
        #[case] rhs_quan: UnitSize,
        #[case] rhs_unit: Unit,
        #[case] exp_cmp: Option<Ordering>,
    ) {
        let lhs = NuDuration {
            quantity: lhs_quan,
            unit: lhs_unit,
        };
        let rhs = NuDuration {
            quantity: rhs_quan,
            unit: rhs_unit,
        };

        assert_eq!(
            exp_cmp,
            lhs.partial_cmp(&rhs),
            "partial cmp: expected matches observed"
        );

        let reversed_exp_cmp = match exp_cmp {
            Some(Ordering::Greater) => Some(Ordering::Less),
            Some(Ordering::Equal) => Some(Ordering::Equal),
            Some(Ordering::Less) => Some(Ordering::Greater),
            None => None,
        };

        assert_eq!(
            reversed_exp_cmp,
            rhs.partial_cmp(&lhs),
            "reversed partial cmp: reversed expected matches observed"
        );
    }
}
