use super::parser::datetime_in_timezone;
use crate::date::TIME_ZONE_NAMES;
use crate::date::utils::parse_date_from_string;
use nu_engine::command_prelude::*;
use nu_protocol::DateTime;

#[derive(Clone)]
pub struct DateToTimezone;

impl Command for DateToTimezone {
    fn name(&self) -> &str {
        "date to-timezone"
    }

    fn signature(&self) -> Signature {
        Signature::build("date to-timezone")
            .input_output_types(vec![
                (Type::Date, Type::Date),
                (Type::String, Type::Date),
                (
                    Type::List(Box::new(Type::Date)),
                    Type::List(Box::new(Type::Date)),
                ),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::Date)),
                ),
            ])
            .allow_variants_without_examples(true) // https://github.com/nushell/nushell/issues/7032
            .param(
                PositionalArg::new("time zone", SyntaxShape::String)
                    .desc("Time zone description.")
                    .completion(Completion::new_list(TIME_ZONE_NAMES.as_slice()))
                    .required(),
            )
            .category(Category::Date)
    }

    fn description(&self) -> &str {
        "Convert a date to a given time zone."
    }

    fn extra_description(&self) -> &str {
        "Use 'date list-timezone' to list all supported time zones."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![
            "tz",
            "transform",
            "convert",
            "UTC",
            "GMT",
            "list",
            "list-timezone",
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let timezone: Spanned<String> = call.req(engine_state, stack, 0)?;

        // This doesn't match explicit nulls
        if let PipelineData::Empty = input {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(
            move |value| helper(value, head, &timezone),
            engine_state.signals(),
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        // Dates at +05:00, as the examples convert to.
        let at_five = |text: &str| {
            DateTime::parse_rfc3339(&format!("{text}+05:00"))
                .ok()
                .map(Value::test_date)
        };
        let example_result_1 = || at_five("2020-10-10T13:00:00");
        let example_result_list_datetime = || {
            Some(Value::test_list(vec![
                at_five("2021-10-22T13:00:00")?,
                at_five("2021-10-23T13:00:00")?,
            ]))
        };
        let example_result_list_string = || {
            Some(Value::test_list(vec![
                at_five("2020-10-10T13:00:00")?,
                at_five("2020-10-10T15:00:00")?,
            ]))
        };

        vec![
            Example {
                description: "Get the current date in UTC+05:00.",
                example: "date now | date to-timezone '+0500'",
                result: None,
            },
            Example {
                description: "Get the current date in the local time zone.",
                example: "date now | date to-timezone local",
                result: None,
            },
            Example {
                description: "Get the current date in Hawaii.",
                example: "date now | date to-timezone US/Hawaii",
                result: None,
            },
            Example {
                description: "Get a date in a different time zone, from a string.",
                example: r#""2020-10-10 10:00:00 +02:00" | date to-timezone "+0500""#,
                result: example_result_1(),
            },
            Example {
                description: "Get a date in a different time zone, from a datetime.",
                example: r#""2020-10-10 10:00:00 +02:00" | into datetime | date to-timezone "+0500""#,
                result: example_result_1(),
            },
            Example {
                description: "Convert a list of datetimes to a given time zone.",
                example: "[2021-10-22T10:00:00+02:00, 2021-10-23T10:00:00+02:00] | date to-timezone \"+0500\"",
                result: example_result_list_datetime(),
            },
            Example {
                description: "Convert a list of date strings to a given time zone.",
                example: r#"["2020-10-10 10:00:00 +02:00", "2020-10-10 12:00:00 +02:00"] | date to-timezone "+0500""#,
                result: example_result_list_string(),
            },
        ]
    }
}

fn helper(value: Value, head: Span, timezone: &Spanned<String>) -> Value {
    let val_span = value.span();
    match value {
        Value::Date { val, .. } => _to_timezone(val, timezone, head),
        Value::String { val, .. } => {
            let time = parse_date_from_string(&val, val_span);
            match time {
                Ok(dt) => _to_timezone(dt, timezone, head),
                Err(e) => e,
            }
        }

        Value::Nothing { .. } => _to_timezone(DateTime::now(), timezone, head),
        _ => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "date, string (that represents datetime), or nothing".into(),
                wrong_type: value.get_type().to_string(),
                dst_span: head,
                src_span: val_span,
            },
            head,
        ),
    }
}

fn _to_timezone(dt: DateTime, timezone: &Spanned<String>, span: Span) -> Value {
    match datetime_in_timezone(dt, timezone.item.as_str()) {
        Ok(dt) => Value::date(dt, span),
        Err(_) => Value::error(
            ShellError::TypeMismatch {
                err_message: String::from("invalid time zone"),
                span: timezone.span,
            },
            timezone.span,
        ),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(DateToTimezone)
    }
}
