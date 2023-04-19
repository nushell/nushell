use chrono::{DateTime, Datelike, Duration, Local};
use nu_protocol::report_error;
use nu_protocol::{
    ast::RangeInclusion,
    engine::{EngineState, Stack, StateWorkingSet},
    Range, ShellError, Span, Value,
};
use std::path::PathBuf;

pub fn get_init_cwd() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| {
        std::env::var("PWD")
            .map(Into::into)
            .unwrap_or_else(|_| nu_path::home_dir().unwrap_or_default())
    })
}

pub fn get_guaranteed_cwd(engine_state: &EngineState, stack: &Stack) -> PathBuf {
    nu_engine::env::current_dir(engine_state, stack).unwrap_or_else(|e| {
        let working_set = StateWorkingSet::new(engine_state);
        report_error(&working_set, &e);
        crate::util::get_init_cwd()
    })
}

type MakeRangeError = fn(&str, Span) -> ShellError;

pub fn process_range(range: &Range) -> Result<(isize, isize), MakeRangeError> {
    let start = match &range.from {
        Value::Int { val, .. } => isize::try_from(*val).unwrap_or_default(),
        Value::Nothing { .. } => 0,
        _ => {
            return Err(|msg, span| ShellError::TypeMismatch {
                err_message: msg.to_string(),
                span,
            })
        }
    };

    let end = match &range.to {
        Value::Int { val, .. } => {
            if matches!(range.inclusion, RangeInclusion::Inclusive) {
                isize::try_from(*val).unwrap_or(isize::max_value())
            } else {
                isize::try_from(*val).unwrap_or(isize::max_value()) - 1
            }
        }
        Value::Nothing { .. } => isize::max_value(),
        _ => {
            return Err(|msg, span| ShellError::TypeMismatch {
                err_message: msg.to_string(),
                span,
            })
        }
    };

    Ok((start, end))
}

pub fn leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

#[allow(clippy::if_same_then_else)]
pub fn parse_year(date: DateTime<Local>, years: i32) -> Duration {
    let month = date.month();
    let (start_year, end_year) = if years < 0 {
        (date.year() + years, date.year())
    } else {
        (date.year(), date.year() + years)
    };

    // find how many leap years are in between the start year with month and day and end year with month and day
    let mut num_of_leap_days = 0;

    for year in start_year..end_year + 1 {
        if leap_year(year) && year == start_year && month < 3 {
            num_of_leap_days += 1
        } else if leap_year(year) && year == end_year && month >= 3 {
            num_of_leap_days += 1
        } else if leap_year(year) && year == end_year && month == 2 && date.day() == 29 {
            num_of_leap_days += 1
        } else if leap_year(year) && month < 3 {
            num_of_leap_days += 1
        }
    }

    let days = 365 * years.abs() + num_of_leap_days;
    if years < 0 {
        let num_days = (-days) as i64;
        Duration::days(num_days)
    } else {
        Duration::days(days.into())
    }
}

pub fn parse_months(date: DateTime<Local>, months: i32) -> Duration {
    let mut days = 0;
    let mut year = date.year();
    let mut month = date.month();

    for _ in 1..=months.abs() {
        // months can be negative but we still want to iterate from 1 to num of months
        days += match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if leap_year(year) => 29,
            2 => 28,
            _ => unreachable!(),
        };

        if months < 0 {
            if month == 1 {
                month = 12;
                year -= 1;
            } else {
                month -= 1
            }
        }
        // else clippy complains
        if months > 0 {
            if month == 12 {
                year += 1;
                month = 1
            } else {
                month += 1
            }
        }
    }

    if months < 0 {
        Duration::days(-days)
    } else {
        Duration::days(days)
    }
}

// adapted from https://github.com/uutils/coreutils/blob/main/src/uu/touch/src/touch.rs
// does not consider daylights savings, and cannot accept months or years yet
/// Parses relative time into a duration
/// e.g., yesterday, 2 hours ago, -2 weeks
pub fn parse_relative_time(s: &str) -> Option<Duration> {
    // Relative time, like "-1 hour" or "+3 days".

    let mut tokens: Vec<&str> = s.split_whitespace().collect();
    let past_time = tokens.contains(&"ago");

    if past_time {
        tokens = tokens[0..tokens.len() - 1].to_vec()
    }

    let result = match &tokens[..] {
        [num_str, "year" | "years"] => num_str
            .parse::<i32>()
            .ok()
            .map(|n| parse_year(Local::now(), n)),

        [num_str, "month" | "months"] => num_str
            .parse::<i32>()
            .ok()
            .map(|n| parse_months(Local::now(), n)),
        ["year" | "years"] => Some(parse_year(Local::now(), 1)),
        ["month" | "months"] => Some(parse_months(Local::now(), 1)),
        [num_str, "fortnight" | "fortnights"] => {
            num_str.parse::<i64>().ok().map(|n| Duration::weeks(2 * n))
        }
        ["fortnight" | "fortnights"] => Some(Duration::weeks(2)),
        [num_str, "week" | "weeks"] => num_str.parse::<i64>().ok().map(Duration::weeks),
        ["week" | "weeks"] => Some(Duration::weeks(1)),
        [num_str, "day" | "days"] => num_str.parse::<i64>().ok().map(Duration::days),
        ["day" | "days"] => Some(Duration::days(1)),
        [num_str, "hour" | "hours"] => num_str.parse::<i64>().ok().map(Duration::hours),
        ["hour" | "hours"] => Some(Duration::hours(1)),
        [num_str, "minute" | "minutes" | "min" | "mins"] => {
            num_str.parse::<i64>().ok().map(Duration::minutes)
        }
        ["minute" | "minutes" | "min" | "mins"] => Some(Duration::minutes(1)),
        [num_str, "second" | "seconds" | "sec" | "secs"] => {
            num_str.parse::<i64>().ok().map(Duration::seconds)
        }
        ["second" | "seconds" | "sec" | "secs"] => Some(Duration::seconds(1)),
        ["now" | "today"] => Some(Duration::nanoseconds(0)),
        ["yesterday"] => Some(Duration::days(-1)),
        ["tomorrow"] => Some(Duration::days(1)),
        _ => None,
    };

    if past_time {
        if let Some(duration) = result {
            let negative = result.filter(|&duration| duration.num_milliseconds() < 0);
            if negative.is_some() {
                Some(duration)
            } else {
                Some(-duration)
            }
        } else {
            None
        }
    } else {
        result
    }
}
