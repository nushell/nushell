use core::fmt::{Display, Write};

use crate::prelude::*;
use chrono::DateTime;
use nu_errors::ShellError;
use nu_protocol::{Dictionary, Value};

use chrono::{Datelike, TimeZone, Timelike};
use indexmap::IndexMap;
use nu_protocol::UntaggedValue;

pub fn date_to_value_raw<T: TimeZone>(dt: DateTime<T>, dt_format: String) -> String
where
    T::Offset: Display,
{
    let result = dt.format(&dt_format);
    format!("{}", result)
}

pub fn date_to_value<T: TimeZone>(
    dt: DateTime<T>,
    tag: Tag,
    dt_format: String,
) -> Result<Value, ShellError>
where
    T::Offset: Display,
{
    let mut indexmap = IndexMap::new();

    if dt_format.is_empty() {
        indexmap.insert(
            "year".to_string(),
            UntaggedValue::int(dt.year()).into_value(&tag),
        );
        indexmap.insert(
            "month".to_string(),
            UntaggedValue::int(dt.month()).into_value(&tag),
        );
        indexmap.insert(
            "day".to_string(),
            UntaggedValue::int(dt.day()).into_value(&tag),
        );
        indexmap.insert(
            "hour".to_string(),
            UntaggedValue::int(dt.hour()).into_value(&tag),
        );
        indexmap.insert(
            "minute".to_string(),
            UntaggedValue::int(dt.minute()).into_value(&tag),
        );
        indexmap.insert(
            "second".to_string(),
            UntaggedValue::int(dt.second()).into_value(&tag),
        );

        let tz = dt.offset();
        let mut tz_formatted = String::new();

        tz_formatted
            .write_fmt(format_args!("{}", tz))
            .map_err(|e| ShellError::untagged_runtime_error(format!("{}", e)))?;

        indexmap.insert(
            "timezone".to_string(),
            UntaggedValue::string(tz_formatted).into_value(&tag),
        );
    } else {
        let result = dt.format(&dt_format);
        let mut result_formatted = String::new();

        result_formatted
            .write_fmt(format_args!("{}", result))
            .map_err(|e| ShellError::untagged_runtime_error(format!("{}", e)))?;

        indexmap.insert(
            "formatted".to_string(),
            UntaggedValue::string(result_formatted).into_value(&tag),
        );
    }

    Ok(UntaggedValue::Row(Dictionary::from(indexmap)).into_value(&tag))
}
