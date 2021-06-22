use super::parser::{datetime_in_timezone, ParseErrorKind};
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Primitive, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;

pub struct Date;

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
        "Convert a date to a given time zone."
    }

    fn extra_usage(&self) -> &str {
        "Use 'date list-timezone' to list all supported time zones."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        to_timezone(args)
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

fn to_timezone(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    let timezone: Tagged<String> = args.req(0)?;

    Ok(args
        .input
        .map(move |value| match value {
            Value {
                value: UntaggedValue::Primitive(Primitive::Date(dt)),
                ..
            } => match datetime_in_timezone(&dt, &timezone.item) {
                Ok(dt) => {
                    let value = UntaggedValue::date(dt).into_value(&tag);

                    Ok(value)
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
        .into_input_stream())
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
