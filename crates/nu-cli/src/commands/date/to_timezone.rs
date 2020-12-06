use crate::commands::date::parser::{get_timezone_offset, ParseErrorKind};
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;

pub struct Date;

#[derive(Deserialize)]
struct DateToTimeZoneArguments {
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
        
Supported time zone abbreviations include: GMT, UTC, UT, EDT, EST, CDT, CST, MDT, MST, PDT, PST, LOCAL.
        "
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        to_timezone(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get the current date in UTC+05:00",
                example: "date now | date to-timezone +0500",
                result: None,
            },
            Example {
                description: "Get the current date in PST",
                example: "date now | date to-timezone PST",
                result: None,
            },
        ]
    }
}

async fn to_timezone(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let tag = args.call_info.name_tag.clone();
    let (DateToTimeZoneArguments { timezone }, input) = args.process(&registry).await?;

    Ok(input
        .map(move |value| match value {
            Value {
                value: UntaggedValue::Primitive(Primitive::Date(dt)),
                ..
            } => match get_timezone_offset(&timezone.item) {
                Ok(tz) => {
                    let value = UntaggedValue::date(dt.with_timezone(&tz)).into_value(&tag);

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
        ParseErrorKind::NotSupported => {
            "The time zone abbreviation is either invalid or not yet supported"
        }
        ParseErrorKind::OutOfRange => "The time zone offset is out of range",
        _ => "The format of the time zone is invalid",
    }
}

#[cfg(test)]
mod tests {
    use super::Date;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        Ok(test_examples(Date {})?)
    }
}
