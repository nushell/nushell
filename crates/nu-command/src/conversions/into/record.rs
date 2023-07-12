use chrono::{DateTime, Datelike, FixedOffset, Timelike};
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    record, Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Type,
    Value,
};
use nu_protocol::{format_duration_as_timeperiod, Record};
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
        vec![
            Example {
                description: "Convert from one row table to record",
                example: "[[value]; [false]] | into record",
                result: Some(Value::test_record(Record {
                    cols: vec!["value".to_string()],
                    vals: vec![Value::test_bool(false)],
                })),
            },
            Example {
                description: "Convert from list to record",
                example: "[1 2 3] | into record",
                result: Some(Value::test_record(Record {
                    cols: vec!["0".to_string(), "1".to_string(), "2".to_string()],
                    vals: vec![Value::test_int(1), Value::test_int(2), Value::test_int(3)],
                })),
            },
            Example {
                description: "Convert from range to record",
                example: "0..2 | into record",
                result: Some(Value::test_record(Record {
                    cols: vec!["0".to_string(), "1".to_string(), "2".to_string()],
                    vals: vec![Value::test_int(0), Value::test_int(1), Value::test_int(2)],
                })),
            },
            Example {
                description: "convert duration to record",
                example: "-500day | into record",
                result: Some(Value::test_record(Record {
                    cols: vec![
                        "year".into(),
                        "month".into(),
                        "week".into(),
                        "day".into(),
                        "sign".into(),
                    ],
                    vals: vec![
                        Value::test_int(1),
                        Value::test_int(4),
                        Value::test_int(2),
                        Value::test_int(1),
                        Value::test_string("-"),
                    ],
                })),
            },
            Example {
                description: "convert record to record",
                example: "{a: 1, b: 2} | into record",
                result: Some(Value::test_record(Record {
                    cols: vec!["a".to_string(), "b".to_string()],
                    vals: vec![Value::test_int(1), Value::test_int(2)],
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
                        Value::test_int(2020),
                        Value::test_int(4),
                        Value::test_int(12),
                        Value::test_int(22),
                        Value::test_int(10),
                        Value::test_int(57),
                        Value::test_string("+02:00"),
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
    let res = match input {
        Value::Date { val, span } => parse_date_into_record(val, span),
        Value::Duration { val, span } => parse_duration_into_record(val, span),
        Value::List { mut vals, span } => match input_type {
            Type::Table(..) if vals.len() == 1 => vals.pop().expect("already checked 1 item"),
            _ => Value::record(
                vals.into_iter()
                    .enumerate()
                    .map(|(i, val)| (i.to_string(), val))
                    .collect(),
                span,
            ),
        },
        Value::Range { val, span } => Value::record(
            val.into_range_iter(engine_state.ctrlc.clone())?
                .enumerate()
                .map(|(i, val)| (i.to_string(), val))
                .collect(),
            span,
        ),
        rec @ Value::Record { .. } => rec,
        Value::Error { .. } => input,
        other => Value::Error {
            error: Box::new(ShellError::OnlySupportsThisInputType {
                exp_input_type: "string".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: call.head,
                src_span: other.expect_span(),
            }),
        },
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
                "month" => "month",
                "yr" => "year",
                _ => "unknown",
            },
            Value::int(split[0].parse::<i64>().unwrap_or(0), span),
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
