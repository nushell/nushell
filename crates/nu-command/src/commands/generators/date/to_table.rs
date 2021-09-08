use crate::prelude::*;
use chrono::{Datelike, Timelike};
use indexmap::IndexMap;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Dictionary, Primitive, Signature, UntaggedValue, Value};

pub struct Date;

impl WholeStreamCommand for Date {
    fn name(&self) -> &str {
        "date to-table"
    }

    fn signature(&self) -> Signature {
        Signature::build("date to-table")
    }

    fn usage(&self) -> &str {
        "Print the date in a structured table."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        to_table(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Print the current date in a table",
            example: "date now | date to-table",
            result: None,
        }]
    }
}

fn to_table(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let input = if args.input.is_empty() {
        InputStream::one(super::now::date_now(&tag))
    } else {
        args.input
    };

    Ok(input
        .map(move |value| match value {
            Value {
                value: UntaggedValue::Primitive(Primitive::Date(dt)),
                ..
            } => {
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
                    UntaggedValue::string(tz.to_string()).into_value(&tag),
                );

                let value = UntaggedValue::Row(Dictionary::from(indexmap)).into_value(&tag);

                Ok(value)
            }
            _ => Err(ShellError::labeled_error(
                "Expected a date from pipeline",
                "requires date input",
                &tag,
            )),
        })
        .into_input_stream())
}

#[cfg(test)]
mod tests {
    use super::Date;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Date {})
    }
}
