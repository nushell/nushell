use chrono::{DateTime, Datelike, FixedOffset, Timelike};
use nu_protocol::format_duration_as_timeperiod;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Type, Value,
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
        "Convert value to record"
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
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        into_record(engine_state, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        let span = Span::test_data();
        vec![
            Example {
                description: "Convert from one row table to record",
                example: "[[value]; [false]] | into record",
                result: Some(Value::Record {
                    cols: vec!["value".to_string()],
                    vals: vec![Value::boolean(false, span)],
                    span,
                }),
            },
            Example {
                description: "Convert from list to record",
                example: "[1 2 3] | into record",
                result: Some(Value::Record {
                    cols: vec!["0".to_string(), "1".to_string(), "2".to_string()],
                    vals: vec![
                        Value::Int { val: 1, span },
                        Value::Int { val: 2, span },
                        Value::Int { val: 3, span },
                    ],
                    span,
                }),
            },
            Example {
                description: "Convert from range to record",
                example: "0..2 | into record",
                result: Some(Value::Record {
                    cols: vec!["0".to_string(), "1".to_string(), "2".to_string()],
                    vals: vec![
                        Value::Int { val: 0, span },
                        Value::Int { val: 1, span },
                        Value::Int { val: 2, span },
                    ],
                    span,
                }),
            },
            Example {
                description: "convert duration to record",
                example: "-500day | into record",
                result: Some(Value::Record {
                    cols: vec![
                        "year".into(),
                        "month".into(),
                        "week".into(),
                        "day".into(),
                        "sign".into(),
                    ],
                    vals: vec![
                        Value::Int { val: 1, span },
                        Value::Int { val: 4, span },
                        Value::Int { val: 2, span },
                        Value::Int { val: 1, span },
                        Value::String {
                            val: "-".into(),
                            span,
                        },
                    ],
                    span,
                }),
            },
            Example {
                description: "convert record to record",
                example: "{a: 1, b: 2} | into record",
                result: Some(Value::Record {
                    cols: vec!["a".to_string(), "b".to_string()],
                    vals: vec![Value::Int { val: 1, span }, Value::Int { val: 2, span }],
                    span,
                }),
            },
            Example {
                description: "convert date to record",
                example: "2020-04-12T22:10:57+02:00 | into record",
                result: Some(Value::Record {
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
                        Value::Int { val: 2020, span },
                        Value::Int { val: 4, span },
                        Value::Int { val: 12, span },
                        Value::Int { val: 22, span },
                        Value::Int { val: 10, span },
                        Value::Int { val: 57, span },
                        Value::String {
                            val: "+02:00".to_string(),
                            span,
                        },
                    ],
                    span,
                }),
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
        Value::Date { val, span } => parse_date_into_record(Ok(val), span),
        Value::Duration { val, span } => parse_duration_into_record(val, span),
        Value::List { mut vals, span } => match input_type {
            Type::Table(..) if vals.len() == 1 => vals.pop().expect("already checked 1 item"),
            _ => {
                let mut cols = vec![];
                let mut values = vec![];
                for (idx, val) in vals.into_iter().enumerate() {
                    cols.push(format!("{idx}"));
                    values.push(val);
                }
                Value::Record {
                    cols,
                    vals: values,
                    span,
                }
            }
        },
        Value::Range { val, span } => {
            let mut cols = vec![];
            let mut vals = vec![];
            for (idx, val) in val.into_range_iter(engine_state.ctrlc.clone())?.enumerate() {
                cols.push(format!("{idx}"));
                vals.push(val);
            }
            Value::Record { cols, vals, span }
        }
        Value::Record { cols, vals, span } => Value::Record { cols, vals, span },
        other => {
            return Err(ShellError::UnsupportedInput(
                "'into record' does not support this input".into(),
                other.span().unwrap_or(call.head),
            ))
        }
    };
    Ok(res.into_pipeline_data())
}

fn parse_date_into_record(date: Result<DateTime<FixedOffset>, Value>, span: Span) -> Value {
    let cols = vec![
        "year".into(),
        "month".into(),
        "day".into(),
        "hour".into(),
        "minute".into(),
        "second".into(),
        "timezone".into(),
    ];
    match date {
        Ok(x) => {
            let vals = vec![
                Value::Int {
                    val: x.year() as i64,
                    span,
                },
                Value::Int {
                    val: x.month() as i64,
                    span,
                },
                Value::Int {
                    val: x.day() as i64,
                    span,
                },
                Value::Int {
                    val: x.hour() as i64,
                    span,
                },
                Value::Int {
                    val: x.minute() as i64,
                    span,
                },
                Value::Int {
                    val: x.second() as i64,
                    span,
                },
                Value::String {
                    val: x.offset().to_string(),
                    span,
                },
            ];
            Value::Record { cols, vals, span }
        }
        Err(e) => e,
    }
}

fn parse_duration_into_record(duration: i64, span: Span) -> Value {
    let (sign, periods) = format_duration_as_timeperiod(duration);

    let mut cols = vec![];
    let mut vals = vec![];
    for p in periods {
        let num_with_unit = p.to_text().to_string();
        let split = num_with_unit.split(' ').collect::<Vec<&str>>();
        cols.push(match split[1] {
            "ns" => "nanosecond".into(),
            "µs" => "microsecond".into(),
            "ms" => "millisecond".into(),
            "sec" => "second".into(),
            "min" => "minute".into(),
            "hr" => "hour".into(),
            "day" => "day".into(),
            "wk" => "week".into(),
            "month" => "month".into(),
            "yr" => "year".into(),
            _ => "unknown".into(),
        });

        vals.push(Value::Int {
            val: split[0].parse::<i64>().unwrap_or(0),
            span,
        });
    }

    cols.push("sign".into());
    vals.push(Value::String {
        val: if sign == -1 { "-".into() } else { "+".into() },
        span,
    });

    Value::Record { cols, vals, span }
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
