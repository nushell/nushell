use crate::commands::date::parser::{datetime_in_timezone, ParseErrorKind};
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;

pub struct Date;

#[derive(Deserialize)]
struct DateToTimeZoneArgs {
    timezone: Tagged<String>,
}

#[async_trait]
impl WholeStreamCommand for Date {
    fn name(&self) -> &str {
        "date to-timezone"
    }

    fn signature(&self) -> Signature {
        Signature::build("date to-timezone").required(
            "time zone",
            SyntaxShape::String,
            "time zone description",
        )
    }

    fn usage(&self) -> &str {
        "Convert a date to a given time zone.
        
Use `date list-timezone` to list all supported time zones.
        "
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        to_timezone(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get the current date in UTC+05:00",
                example: "date now | date to-timezone +0500",
                result: None,
            },
            Example {
                description: "Get the current local date",
                example: "date now | date to-timezone local",
                result: None,
            },
            Example {
                description: "Get the current date in Hawaii",
                example: "date now | date to-timezone US/Hawaii",
                result: None,
            },
        ]
    }
}

async fn to_timezone(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let (DateToTimeZoneArgs { timezone }, input) = args.process().await?;

    Ok(input
        .map(move |value| match value {
            Value {
                value: UntaggedValue::Primitive(Primitive::Date(dt)),
                ..
            } => match datetime_in_timezone(&dt, &timezone.item) {
                Ok(dt) => {
                    let value = UntaggedValue::date(dt).into_value(&tag);

                    ReturnSuccess::value(value)
                }
                Err(e) => Err(ShellError::labeled_error(
                    error_message(e),
                    "invalid time zone",
                    &timezone.tag,
                )),
            },
            _ => Err(ShellError::labeled_error(
                "Expected a date from pipeline",
                "requires date input",
                &tag,
            )),
        })
        .to_output_stream())
}

fn error_message(err: ParseErrorKind) -> &'static str {
    match err {
        ParseErrorKind::Invalid => "The time zone description is invalid",
        ParseErrorKind::OutOfRange => "The time zone offset is out of range",
        ParseErrorKind::TooShort => "The format of the time zone is invalid",
    }
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
