use chrono::{DateTime, Datelike, FixedOffset, Timelike};
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    format_duration_as_timeperiod, record, Category, Example, IntoPipelineData, PipelineData,
    Record, ShellError, Signature, Span, Type, Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "into record"
    }

    fn signature(&self) -> Signature {
        Signature::build("into record")
            .input_output_types(vec![
                (Type::Date, Type::Record(vec![])),
                (Type::Duration, Type::Record(vec![])),
                (Type::List(Box::new(Type::Any)), Type::Record(vec![])),
                (Type::Range, Type::Record(vec![])),
                (Type::Record(vec![]), Type::Record(vec![])),
                (Type::Table(vec![]), Type::Record(vec![])),
            ])
            .category(Category::Conversions)
    }

    fn usage(&self) -> &str {
        "Convert value to record."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        into_record(engine_state, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        let span = Span::test_data();
        vec![
            Example {
                description: "Convert from one row table to record",
                example: "[[value]; [false]] | into record",
                result: Some(Value::test_record(Record {
                    cols: vec!["value".to_string()],
                    vals: vec![Value::bool(false, span)],
                })),
            },
            Example {
                description: "Convert from list to record",
                example: "[1 2 3] | into record",
                result: Some(Value::test_record(Record {
                    cols: vec!["0".to_string(), "1".to_string(), "2".to_string()],
                    vals: vec![
                        Value::int(1, span),
                        Value::int(2, span),
                        Value::int(3, span),
                    ],
                })),
            },
            Example {
                description: "Convert from range to record",
                example: "0..2 | into record",
                result: Some(Value::test_record(Record {
                    cols: vec!["0".to_string(), "1".to_string(), "2".to_string()],
                    vals: vec![
                        Value::int(0, span),
                        Value::int(1, span),
                        Value::int(2, span),
                    ],
                })),
            },
            Example {
                description: "convert duration to record (weeks max)",
                example: "(-500day - 4hr - 5sec) | into record",
                result: Some(Value::test_record(Record {
                    cols: vec![
                        "week".into(),
                        "day".into(),
                        "hour".into(),
                        "second".into(),
                        "sign".into(),
                    ],
                    vals: vec![
                        Value::int(71, span),
                        Value::int(3, span),
                        Value::int(4, span),
                        Value::int(5, span),
                        Value::string("-", span),
                    ],
                })),
            },
            Example {
                description: "convert record to record",
                example: "{a: 1, b: 2} | into record",
                result: Some(Value::test_record(Record {
                    cols: vec!["a".to_string(), "b".to_string()],
                    vals: vec![Value::int(1, span), Value::int(2, span)],
                })),
            },
            Example {
                description: "convert date to record",
                example: "2020-04-12T22:10:57+02:00 | into record",
                result: Some(Value::test_record(Record {
                    cols: vec![
                        "year".into(),
                        "month".into(),
                        "day".into(),
                        "hour".into(),
                        "minute".into(),
                        "second".into(),
                        "timezone".into(),
                    ],
                    vals: vec![
                        Value::int(2020, span),
                        Value::int(4, span),
                        Value::int(12, span),
                        Value::int(22, span),
                        Value::int(10, span),
                        Value::int(57, span),
                        Value::string("+02:00".to_string(), span),
                    ],
                })),
            },
        ]
    }
}

fn into_record(
    engine_state: &EngineState,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let input = input.into_value(call.head);
    let input_type = input.get_type();
    let span = input.span();
    let res = match input {
        Value::Date { val, .. } => parse_date_into_record(val, span),
        Value::Duration { val, .. } => parse_duration_into_record(val, span),
        Value::List { mut vals, .. } => match input_type {
            Type::Table(..) if vals.len() == 1 => vals.pop().expect("already checked 1 item"),
            _ => Value::record(
                vals.into_iter()
                    .enumerate()
                    .map(|(idx, val)| (format!("{idx}"), val))
                    .collect(),
                span,
            ),
        },
        Value::Range { val, .. } => Value::record(
            val.into_range_iter(engine_state.ctrlc.clone())?
                .enumerate()
                .map(|(idx, val)| (format!("{idx}"), val))
                .collect(),
            span,
        ),
        Value::Record { val, .. } => Value::record(val, span),
        Value::Error { .. } => input,
        other => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "string".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: call.head,
                src_span: other.span(),
            },
            call.head,
        ),
    };
    Ok(res.into_pipeline_data())
}

fn parse_date_into_record(date: DateTime<FixedOffset>, span: Span) -> Value {
    Value::record(
        record! {
            "year" => Value::int(date.year() as i64, span),
            "month" => Value::int(date.month() as i64, span),
            "day" => Value::int(date.day() as i64, span),
            "hour" => Value::int(date.hour() as i64, span),
            "minute" => Value::int(date.minute() as i64, span),
            "second" => Value::int(date.second() as i64, span),
            "timezone" => Value::string(date.offset().to_string(), span),
        },
        span,
    )
}

fn parse_duration_into_record(duration: i64, span: Span) -> Value {
    let (sign, periods) = format_duration_as_timeperiod(duration);

    let mut record = Record::new();
    for p in periods {
        let num_with_unit = p.to_text().to_string();
        let split = num_with_unit.split(' ').collect::<Vec<&str>>();
        record.push(
            match split[1] {
                "ns" => "nanosecond",
                "µs" => "microsecond",
                "ms" => "millisecond",
                "sec" => "second",
                "min" => "minute",
                "hr" => "hour",
                "day" => "day",
                "wk" => "week",
                _ => "unknown",
            },
            Value::int(split[0].parse().unwrap_or(0), span),
        );
    }

    record.push(
        "sign",
        Value::string(if sign == -1 { "-" } else { "+" }, span),
    );

    Value::record(record, span)
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
