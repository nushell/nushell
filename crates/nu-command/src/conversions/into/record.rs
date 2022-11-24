use chrono::{DateTime, Datelike, FixedOffset, Timelike};
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
                (Type::Bool, Type::Record(vec![])),
                // (Type::Date, Type::Record(vec![])),
                (Type::Duration, Type::Record(vec![])),
                (Type::Filesize, Type::Record(vec![])),
                (Type::Float, Type::Record(vec![])),
                (Type::Int, Type::Record(vec![])),
                (Type::List(Box::new(Type::Any)), Type::Record(vec![])),
                (Type::Range, Type::Record(vec![])),
                (Type::Record(vec![]), Type::Record(vec![])),
                // (Type::Number, Type::Record(vec![])),
                (Type::String, Type::Record(vec![])),
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
                example: "echo [[value]; [false]] | into record",
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
                description: "Convert bool to record",
                example: "true | into record",
                result: Some(Value::Record {
                    cols: vec!["value".to_string()],
                    vals: vec![Value::boolean(true, span)],
                    span,
                }),
            },
            Example {
                description: "convert integer to record",
                example: "1 | into record",
                result: Some(Value::Record {
                    cols: vec!["value".to_string()],
                    vals: vec![Value::Int { val: 1, span }],
                    span,
                }),
            },
            Example {
                description: "convert decimal to record",
                example: "0.3 | into record",
                result: Some(Value::Record {
                    cols: vec!["value".to_string()],
                    vals: vec![Value::Float { val: 0.3, span }],
                    span,
                }),
            },
            Example {
                description: "convert string to record",
                example: "'true' | into record",
                result: Some(Value::Record {
                    cols: vec!["value".to_string()],
                    vals: vec![Value::String {
                        val: "true".to_string(),
                        span,
                    }],
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
                example: "1sec | into record",
                result: Some(Value::Record {
                    cols: vec!["value".to_string()],
                    vals: vec![Value::Duration {
                        val: 1000 * 1000 * 1000,
                        span,
                    }],
                    span,
                }),
            },
            Example {
                description: "convert filesize to record",
                example: "10b | into record",
                result: Some(Value::Record {
                    cols: vec!["value".to_string()],
                    vals: vec![Value::Filesize { val: 10, span }],
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
            // Couldn't get test harness to accept this
            // Example {
            //     description: "convert date to record",
            //     example: "2020-04-12T22:10:57+0200 | into record",
            //     result: Some(Value::Record {
            //         cols: vec![
            //             "year".into(),
            //             "month".into(),
            //             "day".into(),
            //             "hour".into(),
            //             "minute".into(),
            //             "second".into(),
            //             "timezone".into(),
            //         ],
            //         vals: vec![
            //             Value::Int { val: 2020, span },
            //             Value::Int { val: 4, span },
            //             Value::Int { val: 12, span },
            //             Value::Int { val: 22, span },
            //             Value::Int { val: 10, span },
            //             Value::Int { val: 57, span },
            //             Value::String {
            //                 val: "+02:00".to_string(),
            //                 span,
            //             },
            //         ],
            //         span,
            //     }),
            // },
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
        // Value::Binary
        // Value::Block
        Value::Bool { val, span } => Value::Record {
            cols: vec!["value".into()],
            vals: vec![Value::boolean(val, span)],
            span,
        },
        // Value::CellPath
        // Value::Closure
        // Value::CustomValue
        Value::Date { val, span } => parse_date_into_record(Ok(val), span),
        Value::Duration { val, span } => Value::Record {
            cols: vec!["value".into()],
            vals: vec![Value::Duration { val, span }],
            span,
        },
        // Value::Error
        Value::Filesize { val, span } => Value::Record {
            cols: vec!["value".into()],
            vals: vec![Value::Filesize { val, span }],
            span,
        },
        Value::Float { val, span } => Value::Record {
            cols: vec!["value".into()],
            vals: vec![Value::float(val, span)],
            span,
        },
        Value::Int { val, span } => Value::Record {
            cols: vec!["value".into()],
            vals: vec![Value::int(val, span)],
            span,
        },
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
        // Value::Nothing
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
        Value::String { val, span } => Value::Record {
            cols: vec!["value".into()],
            vals: vec![Value::string(val, span)],
            span,
        },
        other => Value::Error {
            error: ShellError::UnsupportedInput(
                "'into record' does not support this input".into(),
                other.span().unwrap_or(call.head),
            ),
        },
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
