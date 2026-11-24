use std::{
    borrow::Cow,
    fmt::{Display, Formatter},
};

use crate::DurationMaxUnit;

#[derive(Clone, Copy)]
pub enum TimePeriod {
    Nanos(i64),
    Micros(i64),
    Millis(i64),
    Seconds(i64),
    Minutes(i64),
    Hours(i64),
    Days(i64),
    Weeks(i64),
    Months(i64),
    Years(i64),
}

impl TimePeriod {
    pub fn to_text(self) -> Cow<'static, str> {
        match self {
            Self::Nanos(n) => format!("{n} ns").into(),
            Self::Micros(n) => format!("{n} µs").into(),
            Self::Millis(n) => format!("{n} ms").into(),
            Self::Seconds(n) => format!("{n} sec").into(),
            Self::Minutes(n) => format!("{n} min").into(),
            Self::Hours(n) => format!("{n} hr").into(),
            Self::Days(n) => format!("{n} day").into(),
            Self::Weeks(n) => format!("{n} wk").into(),
            Self::Months(n) => format!("{n} month").into(),
            Self::Years(n) => format!("{n} yr").into(),
        }
    }
}

impl Display for TimePeriod {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_text())
    }
}

pub fn format_duration(duration: i64, max_unit: DurationMaxUnit) -> String {
    let (sign, periods) = format_duration_as_timeperiod(duration, max_unit);

    let text = periods
        .into_iter()
        .map(|p| p.to_text().to_string().replace(' ', ""))
        .collect::<Vec<String>>();

    format!(
        "{}{}",
        if sign == -1 { "-" } else { "" },
        text.join(" ").trim()
    )
}

pub fn format_duration_as_timeperiod(
    duration: i64,
    max_unit: DurationMaxUnit,
) -> (i32, Vec<TimePeriod>) {
    // Current duration doesn't know a date it's based on, weeks is the max time unit it can normalize into.
    // Don't guess or estimate how many years or months it might contain.

    let sign = if duration >= 0 { 1 } else { -1 };
    let duration = duration.max(-i64::MAX).abs();

    // Each unit and its length in nanoseconds, largest first. Units above `max_unit` are skipped,
    // so their time is counted in the largest unit allowed.
    type Unit = (DurationMaxUnit, i64, fn(i64) -> TimePeriod);
    let units: [Unit; 8] = [
        (
            DurationMaxUnit::Week,
            7 * 86_400 * 1_000_000_000,
            TimePeriod::Weeks,
        ),
        (
            DurationMaxUnit::Day,
            86_400 * 1_000_000_000,
            TimePeriod::Days,
        ),
        (
            DurationMaxUnit::Hour,
            3_600 * 1_000_000_000,
            TimePeriod::Hours,
        ),
        (
            DurationMaxUnit::Minute,
            60 * 1_000_000_000,
            TimePeriod::Minutes,
        ),
        (DurationMaxUnit::Second, 1_000_000_000, TimePeriod::Seconds),
        (DurationMaxUnit::Millisecond, 1_000_000, TimePeriod::Millis),
        (DurationMaxUnit::Microsecond, 1_000, TimePeriod::Micros),
        (DurationMaxUnit::Nanosecond, 1, TimePeriod::Nanos),
    ];

    let mut periods = vec![];
    let mut remainder = duration;
    for (unit, length, period) in units {
        if max_unit <= unit {
            let wholes = remainder / length;
            remainder -= wholes * length;
            if wholes != 0 {
                periods.push(period(wholes));
            }
        }
    }

    if periods.is_empty() {
        periods.push(TimePeriod::Seconds(0));
    }

    (sign, periods)
}
