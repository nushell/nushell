use crate::prelude::*;
use chrono::DateTime;
use nu_protocol::{Dictionary, Value};

use chrono::{Datelike, TimeZone, Timelike};
use core::fmt::Display;
use indexmap::IndexMap;
use nu_protocol::UntaggedValue;

pub fn date_to_value_raw<T: TimeZone>(dt: DateTime<T>, dt_format: String) -> String
where
    T::Offset: Display,
{
    let result = dt.format(&dt_format);
    format!("{}", result)
}

pub fn date_to_value<T: TimeZone>(dt: DateTime<T>, tag: Tag, dt_format: String) -> Value
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
        indexmap.insert(
            "timezone".to_string(),
            UntaggedValue::string(format!("{}", tz)).into_value(&tag),
        );
    } else {
        let result = dt.format(&dt_format);
        indexmap.insert(
            "formatted".to_string(),
            UntaggedValue::string(format!("{}", result)).into_value(&tag),
        );
    }

    UntaggedValue::Row(Dictionary::from(indexmap)).into_value(&tag)
}
