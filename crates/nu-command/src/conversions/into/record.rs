use chrono::{DateTime, Datelike, FixedOffset, Timelike};
use nu_engine::command_prelude::*;
use nu_protocol::format_duration_as_timeperiod;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "into record"
    }

    fn signature(&self) -> Signature {
        Signature::build("into record")
            .input_output_types(vec![
                (Type::Date, Type::record()),
                (Type::Duration, Type::record()),
                (Type::List(Box::new(Type::Any)), Type::record()),
                (Type::Range, Type::record()),
                (Type::record(), Type::record()),
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
                result: Some(Value::test_record(record! {
                    "value" => Value::test_bool(false),
                })),
            },
            Example {
                description: "Convert from list to record",
                example: "[1 2 3] | into record",
                result: Some(Value::test_record(record! {
                    "0" => Value::test_int(1),
                    "1" => Value::test_int(2),
                    "2" => Value::test_int(3),
                })),
            },
            Example {
                description: "Convert from range to record",
                example: "0..2 | into record",
                result: Some(Value::test_record(record! {
                    "0" => Value::test_int(0),
                    "1" => Value::test_int(1),
                    "2" => Value::test_int(2),
                })),
            },
            Example {
                description: "convert duration to record (weeks max)",
                example: "(-500day - 4hr - 5sec) | into record",
                result: Some(Value::test_record(record! {
                    "week" =>   Value::test_int(71),
                    "day" =>    Value::test_int(3),
                    "hour" =>   Value::test_int(4),
                    "second" => Value::test_int(5),
                    "sign" =>   Value::test_string("-"),
                })),
            },
            Example {
                description: "convert record to record",
                example: "{a: 1, b: 2} | into record",
                result: Some(Value::test_record(record! {
                    "a" =>  Value::test_int(1),
                    "b" =>  Value::test_int(2),
                })),
            },
            Example {
                description: "convert date to record",
                example: "2020-04-12T22:10:57+02:00 | into record",
                result: Some(Value::test_record(record! {
                    "year" =>     Value::test_int(2020),
                    "month" =>    Value::test_int(4),
                    "day" =>      Value::test_int(12),
                    "hour" =>     Value::test_int(22),
                    "minute" =>   Value::test_int(10),
                    "second" =>   Value::test_int(57),
                    "timezone" => Value::test_string("+02:00"),
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
            val.into_range_iter(span, engine_state.ctrlc.clone())
                .enumerate()
                .map(|(idx, val)| (format!("{idx}"), val))
                .collect(),
            span,
        ),
        Value::Record { .. } => input,
        Value::Error { .. } => input,
        other => Value::error(
            ShellError::TypeMismatch {
                err_message: format!("Can't convert {} to record", other.get_type()),
                span: other.span(),
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
