use crate::prelude::*;
use chrono::{DateTime, Local, Utc};
use nu_errors::ShellError;
use nu_protocol::{Dictionary, Value};

use crate::commands::WholeStreamCommand;
use chrono::{Datelike, TimeZone, Timelike};
use core::fmt::Display;
use indexmap::IndexMap;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue};

pub struct Date;

#[async_trait]
impl WholeStreamCommand for Date {
    fn name(&self) -> &str {
        "date"
    }

    fn signature(&self) -> Signature {
        Signature::build("date")
            .switch("utc", "use universal time (UTC)", Some('u'))
            .switch("local", "use the local time", Some('l'))
            .named(
                "format",
                SyntaxShape::String,
                "report datetime in supplied strftime format",
                Some('f'),
            )
            .switch("raw", "print date without tables", Some('r'))
    }

    fn usage(&self) -> &str {
        "Get the current datetime."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        date(args, registry).await
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
            Example {
                description: "Get the current time and date and report it based on format",
                example: "date --format '%Y-%m-%d %H:%M:%S.%f %z'",
                result: None,
            },
            Example {
                description: "Get the current time and date and report it without a table",
                example: "date --format '%Y-%m-%d %H:%M:%S.%f %z' --raw",
                result: None,
            },
        ]
    }
}

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

pub async fn date(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let args = args.evaluate_once(&registry).await?;
    let tag = args.call_info.name_tag.clone();
    let raw = args.has("raw");

    let dt_fmt = if args.has("format") {
        if let Some(dt_fmt) = args.get("format") {
            dt_fmt.convert_to_string()
        } else {
            "".to_string()
        }
    } else {
        "".to_string()
    };

    let value = if args.has("utc") {
        let utc: DateTime<Utc> = Utc::now();
        if raw {
            UntaggedValue::string(date_to_value_raw(utc, dt_fmt)).into_untagged_value()
        } else {
            date_to_value(utc, tag, dt_fmt)
        }
    } else {
        let local: DateTime<Local> = Local::now();
        if raw {
            UntaggedValue::string(date_to_value_raw(local, dt_fmt)).into_untagged_value()
        } else {
            date_to_value(local, tag, dt_fmt)
        }
    };

    Ok(OutputStream::one(value))
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
