use chrono::{Local, TimeZone};
use human_date_parser::{from_human_time, ParseResult};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct DateFromHuman;

impl Command for DateFromHuman {
    fn name(&self) -> &str {
        "date from-human"
    }

    fn signature(&self) -> Signature {
        Signature::build("date from-human")
            .input_output_types(vec![
                (Type::String, Type::Date),
                (Type::Nothing, Type::table()),
            ])
            .allow_variants_without_examples(false)
            .switch(
                "list-human",
                "Show human-readable datetime parsing examples",
                Some('n'),
            )
            .category(Category::Date)
    }

    fn description(&self) -> &str {
        "Convert a date formatted as a 'humanized' string to a datetime."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![
            "relative",
            "now",
            "today",
            "tomorrow",
            "yesterday",
            "weekday",
            "weekday_name",
            "timezone",
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        if call.has_flag(engine_state, stack, "list-human")? {
            return Ok(list_human_readable_examples(call.head).into_pipeline_data());
        }
        let head = call.head;
        // This doesn't match explicit nulls
        if matches!(input, PipelineData::Empty) {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(move |value| helper(value, head), engine_state.signals())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Parsing human readable datetime",
                example: "'Today at 18:30' | into datetime",
                result: None,
            },
            Example {
                description: "Parsing human readable datetime",
                example: "'Last Friday at 19:45' | into datetime",
                result: None,
            },
            Example {
                description: "Parsing human readable datetime",
                example: "'In 5 minutes and 30 seconds' | into datetime",
                result: None,
            },
        ]
    }
}

fn helper(value: Value, head: Span) -> Value {
    let span = value.span();
    let input_val = match value {
        Value::String { val, .. } => val,
        other => {
            return Value::error(
                ShellError::OnlySupportsThisInputType {
                    exp_input_type: "string".to_string(),
                    wrong_type: other.get_type().to_string(),
                    dst_span: head,
                    src_span: span,
                },
                span,
            )
        }
    };

    if let Ok(date) = from_human_time(&input_val, Local::now().naive_local()) {
        match date {
            ParseResult::Date(date) => {
                let time = Local::now().time();
                let combined = date.and_time(time);
                let local_offset = *Local::now().offset();
                let dt_fixed = TimeZone::from_local_datetime(&local_offset, &combined)
                    .single()
                    .unwrap_or_default();
                return Value::date(dt_fixed, span);
            }
            ParseResult::DateTime(date) => {
                let local_offset = *Local::now().offset();
                let dt_fixed = match local_offset.from_local_datetime(&date) {
                    chrono::LocalResult::Single(dt) => dt,
                    chrono::LocalResult::Ambiguous(_, _) => {
                        return Value::error(
                            ShellError::DatetimeParseError {
                                msg: "Ambiguous datetime".to_string(),
                                span,
                            },
                            span,
                        );
                    }
                    chrono::LocalResult::None => {
                        return Value::error(
                            ShellError::DatetimeParseError {
                                msg: "Invalid datetime".to_string(),
                                span,
                            },
                            span,
                        );
                    }
                };
                return Value::date(dt_fixed, span);
            }
            ParseResult::Time(time) => {
                let date = Local::now().date_naive();
                let combined = date.and_time(time);
                let local_offset = *Local::now().offset();
                let dt_fixed = TimeZone::from_local_datetime(&local_offset, &combined)
                    .single()
                    .unwrap_or_default();
                return Value::date(dt_fixed, span);
            }
        }
    }

    match from_human_time(&input_val, Local::now().naive_local()) {
        Ok(date) => match date {
            ParseResult::Date(date) => {
                let time = Local::now().time();
                let combined = date.and_time(time);
                let local_offset = *Local::now().offset();
                let dt_fixed = TimeZone::from_local_datetime(&local_offset, &combined)
                    .single()
                    .unwrap_or_default();
                Value::date(dt_fixed, span)
            }
            ParseResult::DateTime(date) => {
                let local_offset = *Local::now().offset();
                let dt_fixed = match local_offset.from_local_datetime(&date) {
                    chrono::LocalResult::Single(dt) => dt,
                    chrono::LocalResult::Ambiguous(_, _) => {
                        return Value::error(
                            ShellError::DatetimeParseError {
                                msg: "Ambiguous datetime".to_string(),
                                span,
                            },
                            span,
                        );
                    }
                    chrono::LocalResult::None => {
                        return Value::error(
                            ShellError::DatetimeParseError {
                                msg: "Invalid datetime".to_string(),
                                span,
                            },
                            span,
                        );
                    }
                };
                Value::date(dt_fixed, span)
            }
            ParseResult::Time(time) => {
                let date = Local::now().date_naive();
                let combined = date.and_time(time);
                let local_offset = *Local::now().offset();
                let dt_fixed = TimeZone::from_local_datetime(&local_offset, &combined)
                    .single()
                    .unwrap_or_default();
                Value::date(dt_fixed, span)
            }
        },
        Err(_) => Value::error(
            ShellError::IncorrectValue {
                msg: "Cannot parse as humanized date".to_string(),
                val_span: head,
                call_span: span,
            },
            span,
        ),
    }
}

fn list_human_readable_examples(span: Span) -> Value {
    let examples: Vec<String> = vec![
        "Today 18:30".into(),
        "2022-11-07 13:25:30".into(),
        "15:20 Friday".into(),
        "This Friday 17:00".into(),
        "13:25, Next Tuesday".into(),
        "Last Friday at 19:45".into(),
        "In 3 days".into(),
        "In 2 hours".into(),
        "10 hours and 5 minutes ago".into(),
        "1 years ago".into(),
        "A year ago".into(),
        "A month ago".into(),
        "A week ago".into(),
        "A day ago".into(),
        "An hour ago".into(),
        "A minute ago".into(),
        "A second ago".into(),
        "Now".into(),
    ];

    let records = examples
        .iter()
        .map(|s| {
            Value::record(
                record! {
                    "parseable human datetime examples" => Value::test_string(s.to_string()),
                    "result" => helper(Value::test_string(s.to_string()), span),
                },
                span,
            )
        })
        .collect::<Vec<Value>>();

    Value::list(records, span)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(DateFromHuman {})
    }
}
