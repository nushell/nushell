use crate::prelude::*;
use chrono::{DateTime, Local, Utc};
use nu_errors::ShellError;
use nu_protocol::{Dictionary, Value};

use crate::commands::WholeStreamCommand;
use chrono::{Datelike, TimeZone, Timelike};
use core::fmt::Display;
use indexmap::IndexMap;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};

pub struct Date;

impl WholeStreamCommand for Date {
    fn name(&self) -> &str {
        "date"
    }

    fn signature(&self) -> Signature {
        Signature::build("date")
            .switch("utc", "use universal time (UTC)", Some('u'))
            .switch("local", "use the local time", Some('l'))
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

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get the current local time and date",
                example: "date",
                result: None,
            },
            Example {
                description: "Get the current UTC time and date",
                example: "date --utc",
                result: None,
            },
        ]
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
    let registry = registry.clone();
    let stream = async_stream! {
        let args = args.evaluate_once(&registry).await?;

        let tag = args.call_info.name_tag.clone();

        let value = if args.has("utc") {
            let utc: DateTime<Utc> = Utc::now();
            date_to_value(utc, tag)
        } else {
            let local: DateTime<Local> = Local::now();
            date_to_value(local, tag)
        };

        yield ReturnSuccess::value(value);
    };

    Ok(stream.to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Date;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Date {})
    }
}
