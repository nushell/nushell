use std::sync::LazyLock;

use super::parser::datetime_in_timezone;
use crate::date::utils::parse_date_from_string;
use chrono::{DateTime, FixedOffset, Local, LocalResult, TimeZone};
use nu_engine::command_prelude::*;

static TIMEZONES: LazyLock<Vec<&'static str>> =
    LazyLock::new(|| chrono_tz::TZ_VARIANTS.iter().map(|tz| tz.name()).collect());

#[derive(Clone)]
pub struct DateToTimezone;

impl Command for DateToTimezone {
    fn name(&self) -> &str {
        "date to-timezone"
    }

    fn signature(&self) -> Signature {
        Signature::build("date to-timezone")
            .input_output_types(vec![(Type::Date, Type::Date), (Type::String, Type::Date)])
            .allow_variants_without_examples(true) // https://github.com/nushell/nushell/issues/7032
            .param(
                PositionalArg::new("time zone", SyntaxShape::String)
                    .desc("Time zone description.")
                    .completion(Completion::new_list(TIMEZONES.as_slice()))
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
        let example_result_1 = || match FixedOffset::east_opt(5 * 3600)
            .expect("to timezone: help example is invalid")
            .with_ymd_and_hms(2020, 10, 10, 13, 00, 00)
        {
            LocalResult::Single(dt) => Some(Value::date(dt, Span::test_data())),
            _ => panic!("to timezone: help example is invalid"),
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

        Value::Nothing { .. } => {
            let dt = Local::now();
            _to_timezone(dt.with_timezone(dt.offset()), timezone, head)
        }
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

fn _to_timezone(dt: DateTime<FixedOffset>, timezone: &Spanned<String>, span: Span) -> Value {
    match datetime_in_timezone(&dt, timezone.item.as_str()) {
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
    fn test_examples() {
        use crate::test_examples;

        test_examples(DateToTimezone {})
    }
}
