mod as_date;
mod as_datetime;
mod convert_time_zone;
mod datepart;
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
use nu_protocol::{ShellError, Span};
use polars::prelude::{PlSmallStr, TimeZone};
pub use replace_time_zone::ReplaceTimeZone;
pub use strftime::StrFTime;
pub use truncate::Truncate;

pub(crate) fn datetime_commands() -> Vec<Box<dyn PluginCommand<Plugin = PolarsPlugin>>> {
    vec![
        Box::new(AsDate),
        Box::new(AsDateTime),
        Box::new(ConvertTimeZone),
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
        .map_err(|e| ShellError::GenericError {
            error: format!("Invalid timezone: {zone_str} : {e}"),
            msg: "".into(),
            span,
            help: None,
            inner: vec![],
        })?
        .ok_or(ShellError::GenericError {
            error: format!("Invalid timezone {zone_str}"),
            msg: "".into(),
            span,
            help: None,
            inner: vec![],
        })
}

pub fn timezone_utc() -> TimeZone {
    TimeZone::opt_try_new(Some(PlSmallStr::from_str("UTC")))
        .expect("UTC timezone should always be valid")
        .expect("UTC timezone should always be present")
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
