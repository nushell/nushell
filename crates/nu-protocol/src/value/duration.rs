use chrono::Duration;
use std::{
    borrow::Cow,
    fmt::{Display, Formatter},
};

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

pub fn format_duration(duration: i64) -> String {
    let (sign, periods) = format_duration_as_timeperiod(duration);

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

pub fn format_duration_as_timeperiod(duration: i64) -> (i32, Vec<TimePeriod>) {
    // Attribution: most of this is taken from chrono-humanize-rs. Thanks!
    // https://gitlab.com/imp/chrono-humanize-rs/-/blob/master/src/humantime.rs
    // Current duration doesn't know a date it's based on, weeks is the max time unit it can normalize into.
    // Don't guess or estimate how many years or months it might contain.

    let (sign, duration) = if duration >= 0 {
        (1, duration)
    } else {
        (-1, -duration)
    };

    let dur = Duration::nanoseconds(duration);

    /// Split this a duration into number of whole weeks and the remainder
    fn split_weeks(duration: Duration) -> (Option<i64>, Duration) {
        let weeks = duration.num_weeks();
        normalize_split(weeks, Duration::try_weeks(weeks), duration)
    }

    /// Split this a duration into number of whole days and the remainder
    fn split_days(duration: Duration) -> (Option<i64>, Duration) {
        let days = duration.num_days();
        normalize_split(days, Duration::try_days(days), duration)
    }

    /// Split this a duration into number of whole hours and the remainder
    fn split_hours(duration: Duration) -> (Option<i64>, Duration) {
        let hours = duration.num_hours();
        normalize_split(hours, Duration::try_hours(hours), duration)
    }

    /// Split this a duration into number of whole minutes and the remainder
    fn split_minutes(duration: Duration) -> (Option<i64>, Duration) {
        let minutes = duration.num_minutes();
        normalize_split(minutes, Duration::try_minutes(minutes), duration)
    }

    /// Split this a duration into number of whole seconds and the remainder
    fn split_seconds(duration: Duration) -> (Option<i64>, Duration) {
        let seconds = duration.num_seconds();
        normalize_split(seconds, Duration::try_seconds(seconds), duration)
    }

    /// Split this a duration into number of whole milliseconds and the remainder
    fn split_milliseconds(duration: Duration) -> (Option<i64>, Duration) {
        let millis = duration.num_milliseconds();
        normalize_split(millis, Duration::try_milliseconds(millis), duration)
    }

    /// Split this a duration into number of whole seconds and the remainder
    fn split_microseconds(duration: Duration) -> (Option<i64>, Duration) {
        let micros = duration.num_microseconds().unwrap_or_default();
        normalize_split(micros, Duration::microseconds(micros), duration)
    }

    /// Split this a duration into number of whole seconds and the remainder
    fn split_nanoseconds(duration: Duration) -> (Option<i64>, Duration) {
        let nanos = duration.num_nanoseconds().unwrap_or_default();
        normalize_split(nanos, Duration::nanoseconds(nanos), duration)
    }

    fn normalize_split(
        wholes: i64,
        wholes_duration: impl Into<Option<Duration>>,
        total_duration: Duration,
    ) -> (Option<i64>, Duration) {
        match wholes_duration.into() {
            Some(wholes_duration) if wholes != 0 => {
                (Some(wholes), total_duration - wholes_duration)
            }
            _ => (None, total_duration),
        }
    }

    let mut periods = vec![];

    let (weeks, remainder) = split_weeks(dur);
    if let Some(weeks) = weeks {
        periods.push(TimePeriod::Weeks(weeks));
    }

    let (days, remainder) = split_days(remainder);
    if let Some(days) = days {
        periods.push(TimePeriod::Days(days));
    }

    let (hours, remainder) = split_hours(remainder);
    if let Some(hours) = hours {
        periods.push(TimePeriod::Hours(hours));
    }

    let (minutes, remainder) = split_minutes(remainder);
    if let Some(minutes) = minutes {
        periods.push(TimePeriod::Minutes(minutes));
    }

    let (seconds, remainder) = split_seconds(remainder);
    if let Some(seconds) = seconds {
        periods.push(TimePeriod::Seconds(seconds));
    }

    let (millis, remainder) = split_milliseconds(remainder);
    if let Some(millis) = millis {
        periods.push(TimePeriod::Millis(millis));
    }

    let (micros, remainder) = split_microseconds(remainder);
    if let Some(micros) = micros {
        periods.push(TimePeriod::Micros(micros));
    }

    let (nanos, _remainder) = split_nanoseconds(remainder);
    if let Some(nanos) = nanos {
        periods.push(TimePeriod::Nanos(nanos));
    }

    if periods.is_empty() {
        periods.push(TimePeriod::Seconds(0));
    }

    (sign, periods)
}
