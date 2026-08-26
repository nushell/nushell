mod as_date;
mod as_datetime;
mod convert_time_zone;
mod date_range;
mod date_ranges;
mod datepart;
mod datetime_range;
mod datetime_ranges;
mod get_day;
mod get_hour;
mod get_minute;
mod get_month;
mod get_nanosecond;
mod get_ordinal;
mod get_second;
mod get_week;
mod get_weekday;
mod get_year;
mod replace_time_zone;
mod strftime;
mod truncate;

use crate::PolarsPlugin;
use nu_plugin::PluginCommand;

pub use as_date::AsDate;
pub use as_datetime::AsDateTime;
pub use convert_time_zone::ConvertTimeZone;
pub use datepart::ExprDatePart;
pub use get_day::GetDay;
pub use get_hour::GetHour;
pub use get_minute::GetMinute;
pub use get_month::GetMonth;
pub use get_nanosecond::GetNanosecond;
pub use get_ordinal::GetOrdinal;
pub use get_second::GetSecond;
pub use get_week::GetWeek;
pub use get_weekday::GetWeekDay;
pub use get_year::GetYear;
use nu_protocol::shell_error::generic::GenericError;
use nu_protocol::{ShellError, Span, Value};
use polars::prelude::{ClosedWindow, PlSmallStr, TimeZone};
use polars::time::Duration;
pub use replace_time_zone::ReplaceTimeZone;
pub use strftime::StrFTime;
pub use truncate::Truncate;

pub(crate) fn datetime_commands() -> Vec<Box<dyn PluginCommand<Plugin = PolarsPlugin>>> {
    vec![
        Box::new(AsDate),
        Box::new(AsDateTime),
        Box::new(ConvertTimeZone),
        Box::new(date_range::DateRange),
        Box::new(date_ranges::DateRange),
        Box::new(datetime_range::DatetimeRange),
        Box::new(datetime_ranges::DatetimeRanges),
        Box::new(ExprDatePart),
        Box::new(GetDay),
        Box::new(GetHour),
        Box::new(GetMinute),
        Box::new(GetMonth),
        Box::new(GetNanosecond),
        Box::new(GetOrdinal),
        Box::new(GetSecond),
        Box::new(GetWeek),
        Box::new(GetWeekDay),
        Box::new(GetYear),
        Box::new(ReplaceTimeZone),
        Box::new(StrFTime),
        Box::new(Truncate),
    ]
}

pub fn timezone_from_str(zone_str: &str, span: Option<Span>) -> Result<TimeZone, ShellError> {
    TimeZone::opt_try_new(Some(PlSmallStr::from_str(zone_str)))
        .map_err(|e| {
            ShellError::Generic(GenericError::new(
                format!("Invalid timezone: {zone_str} : {e}"),
                "",
                span.unwrap_or_else(Span::unknown),
            ))
        })?
        .ok_or_else(|| {
            ShellError::Generic(GenericError::new(
                format!("Invalid timezone {zone_str}"),
                "",
                span.unwrap_or_else(Span::unknown),
            ))
        })
}

pub fn timezone_utc() -> TimeZone {
    TimeZone::opt_try_new(Some(PlSmallStr::from_str("UTC")))
        .expect("UTC timezone should always be valid")
        .expect("UTC timezone should always be present")
}

pub fn value_to_duration(value: &Value) -> Result<Duration, ShellError> {
    match value {
        Value::Duration { val, .. } => Ok(Duration::new(*val)),
        Value::String { val, .. } => Ok(Duration::try_parse(&val.to_string()).map_err(|e| {
            ShellError::Generic(GenericError::new(
                "Failed to parse duration",
                format!("Failed to parse duration: {}", e),
                value.span(),
            ))
        })?),
        _ => Err(ShellError::Generic(GenericError::new(
            "Invalid duration",
            format!("Expected a string for duration: received {value:?}"),
            value.span(),
        ))),
    }
}

pub fn str_to_closed_window(closed_str: &str, span: Span) -> Result<ClosedWindow, ShellError> {
    match closed_str {
        "left" => Ok(ClosedWindow::Left),
        "right" => Ok(ClosedWindow::Right),
        "both" => Ok(ClosedWindow::Both),
        "none" => Ok(ClosedWindow::None),
        _ => Err(ShellError::Generic(GenericError::new(
            "Invalid closed window",
            format!("Invalid closed window: {}", closed_str),
            span,
        ))),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_timezone_from_str() -> Result<(), ShellError> {
        let tz = timezone_from_str("America/New_York", None)?;
        assert_eq!(tz.to_string(), "America/New_York");
        Ok(())
    }

    #[test]
    fn test_timezone_utc() {
        let tz = timezone_utc();
        assert_eq!(tz.to_string(), "UTC");
    }
}
