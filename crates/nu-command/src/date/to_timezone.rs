use super::parser::datetime_in_timezone;
use crate::date::utils::parse_date_from_string;
use chrono::{DateTime, Local};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Type, Value,
};

use chrono::{FixedOffset, TimeZone};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "date to-timezone"
    }

    fn signature(&self) -> Signature {
        Signature::build("date to-timezone")
            .input_output_types(vec![(Type::Date, Type::Date), (Type::String, Type::Date)])
            .allow_variants_without_examples(true) // https://github.com/nushell/nushell/issues/7032
            .required("time zone", SyntaxShape::String, "time zone description")
            .category(Category::Date)
    }

    fn usage(&self) -> &str {
        "Convert a date to a given time zone."
    }

    fn extra_usage(&self) -> &str {
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
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let head = call.head;
        let timezone: Spanned<String> = call.req(engine_state, stack, 0)?;

        //Ok(PipelineData::new())
        input.map(
            move |value| helper(value, head, &timezone),
            engine_state.ctrlc.clone(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        let example_result_1 = || {
            let dt = FixedOffset::east(5 * 3600)
                .ymd(2020, 10, 10)
                .and_hms(13, 00, 00);
            Some(Value::Date {
                val: dt,
                span: Span::test_data(),
            })
        };

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
            Example {
                description: "Get the current date in Hawaii",
                example: r#""2020-10-10 10:00:00 +02:00" | date to-timezone "+0500""#,
                result: example_result_1(),
            },
            // TODO: This should work but does not; see https://github.com/nushell/nushell/issues/7032
            // Example {
            //     description: "Get the current date in Hawaii, from a datetime object",
            //     example: r#""2020-10-10 10:00:00 +02:00" | into datetime | date to-timezone "+0500""#,
            //     result: example_result_1(),
            // },
        ]
    }
}

fn helper(value: Value, head: Span, timezone: &Spanned<String>) -> Value {
    match value {
        Value::Date { val, span: _ } => _to_timezone(val, timezone, head),
        Value::String {
            val,
            span: val_span,
        } => {
            let time = parse_date_from_string(&val, val_span);
            match time {
                Ok(dt) => _to_timezone(dt, timezone, head),
                Err(e) => e,
            }
        }

        Value::Nothing { span: _ } => {
            let dt = Local::now();
            _to_timezone(dt.with_timezone(dt.offset()), timezone, head)
        }
        _ => Value::Error {
            error: ShellError::DatetimeParseError(head),
        },
    }
}

fn _to_timezone(dt: DateTime<FixedOffset>, timezone: &Spanned<String>, span: Span) -> Value {
    match datetime_in_timezone(&dt, timezone.item.as_str()) {
        Ok(dt) => Value::Date { val: dt, span },
        Err(_) => Value::Error {
            error: ShellError::UnsupportedInput(String::from("invalid time zone"), timezone.span),
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
