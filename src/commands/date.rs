use crate::data::{Dictionary, Value};
use crate::errors::ShellError;
use crate::prelude::*;
use chrono::{DateTime, Local, Utc};

use crate::commands::WholeStreamCommand;
use crate::parser::registry::Signature;
use chrono::{Datelike, TimeZone, Timelike};
use core::fmt::Display;
use indexmap::IndexMap;

pub struct Date;

impl WholeStreamCommand for Date {
    fn name(&self) -> &str {
        "date"
    }

    fn signature(&self) -> Signature {
        Signature::build("date").switch("utc").switch("local")
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

pub fn date_to_value<T: TimeZone>(dt: DateTime<T>, tag: Tag) -> Tagged<Value>
where
    T::Offset: Display,
{
    let mut indexmap = IndexMap::new();

    indexmap.insert("year".to_string(), Value::int(dt.year()).tagged(&tag));
    indexmap.insert("month".to_string(), Value::int(dt.month()).tagged(&tag));
    indexmap.insert("day".to_string(), Value::int(dt.day()).tagged(&tag));
    indexmap.insert("hour".to_string(), Value::int(dt.hour()).tagged(&tag));
    indexmap.insert("minute".to_string(), Value::int(dt.minute()).tagged(&tag));
    indexmap.insert("second".to_string(), Value::int(dt.second()).tagged(&tag));

    let tz = dt.offset();
    indexmap.insert(
        "timezone".to_string(),
        Value::string(format!("{}", tz)).tagged(&tag),
    );

    Value::Row(Dictionary::from(indexmap)).tagged(&tag)
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
