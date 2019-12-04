use crate::prelude::*;
use chrono::{DateTime, Local, Utc};
use nu_errors::ShellError;
use nu_protocol::{Dictionary, Value};

use crate::commands::WholeStreamCommand;
use chrono::{Datelike, TimeZone, Timelike};
use core::fmt::Display;
use indexmap::IndexMap;
use nu_protocol::{Signature, UntaggedValue};

pub struct Date;

impl WholeStreamCommand for Date {
    fn name(&self) -> &str {
        "date"
    }

    fn signature(&self) -> Signature {
        Signature::build("date")
            .switch("utc", "use universal time (UTC)")
            .switch("local", "use the local time")
    }

    fn usage(&self) -> &str {
        "Get the current datetime."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        date(args, registry)
    }
}

pub fn date_to_value<T: TimeZone>(dt: DateTime<T>, tag: Tag) -> Value
where
    T::Offset: Display,
{
    let mut indexmap = IndexMap::new();

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

    UntaggedValue::Row(Dictionary::from(indexmap)).into_value(&tag)
}

pub fn date(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;

    let mut date_out = VecDeque::new();
    let tag = args.call_info.name_tag.clone();

    let value = if args.has("utc") {
        let utc: DateTime<Utc> = Utc::now();
        date_to_value(utc, tag)
    } else {
        let local: DateTime<Local> = Local::now();
        date_to_value(local, tag)
    };

    date_out.push_back(value);

    Ok(date_out.to_output_stream())
}
