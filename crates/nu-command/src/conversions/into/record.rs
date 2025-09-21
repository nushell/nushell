use chrono::{DateTime, Datelike, FixedOffset, Timelike};
use nu_engine::command_prelude::*;
use nu_protocol::format_duration_as_timeperiod;

#[derive(Clone)]
pub struct IntoRecord;

impl Command for IntoRecord {
    fn name(&self) -> &str {
        "into record"
    }

    fn signature(&self) -> Signature {
        Signature::build("into record")
            .input_output_types(vec![
                (Type::Date, Type::record()),
                (Type::Duration, Type::record()),
                (Type::List(Box::new(Type::Any)), Type::record()),
                (Type::record(), Type::record()),
            ])
            .category(Category::Conversions)
    }

    fn description(&self) -> &str {
        "Convert value to record."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert"]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        into_record(call, input)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Convert from one row table to record",
                example: "[[value]; [false]] | into record",
                result: Some(Value::test_record(record! {
                    "value" => Value::test_bool(false),
                })),
            },
            Example {
                description: "Convert from list of records to record",
                example: "[{foo: bar} {baz: quux}] | into record",
                result: Some(Value::test_record(record! {
                    "foo" => Value::test_string("bar"),
                    "baz" => Value::test_string("quux"),
                })),
            },
            Example {
                description: "Convert from list of pairs into record",
                example: "[[foo bar] [baz quux]] | into record",
                result: Some(Value::test_record(record! {
                    "foo" => Value::test_string("bar"),
                    "baz" => Value::test_string("quux"),
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
                    "millisecond" => Value::test_int(0),
                    "microsecond" => Value::test_int(0),
                    "nanosecond" => Value::test_int(0),
                    "timezone" => Value::test_string("+02:00"),
                })),
            },
            Example {
                description: "convert date components to table columns",
                example: "2020-04-12T22:10:57+02:00 | into record | transpose | transpose -r",
                result: None,
            },
        ]
    }
}

fn into_record(call: &Call, input: PipelineData) -> Result<PipelineData, ShellError> {
    let span = input.span().unwrap_or(call.head);
    match input {
        PipelineData::Value(Value::Date { val, .. }, _) => {
            Ok(parse_date_into_record(val, span).into_pipeline_data())
        }
        PipelineData::Value(Value::Duration { val, .. }, _) => {
            Ok(parse_duration_into_record(val, span).into_pipeline_data())
        }
        PipelineData::Value(Value::List { .. }, _) | PipelineData::ListStream(..) => {
            let mut record = Record::new();
            let metadata = input.metadata();

            enum ExpectedType {
                Record,
                Pair,
            }
            let mut expected_type = None;

            for item in input.into_iter() {
                let span = item.span();
                match item {
                    Value::Record { val, .. }
                        if matches!(expected_type, None | Some(ExpectedType::Record)) =>
                    {
                        // Don't use .extend() unless that gets changed to check for duplicate keys
                        for (key, val) in val.into_owned() {
                            record.insert(key, val);
                        }
                        expected_type = Some(ExpectedType::Record);
                    }
                    Value::List { mut vals, .. }
                        if matches!(expected_type, None | Some(ExpectedType::Pair)) =>
                    {
                        if vals.len() == 2 {
                            let (val, key) = vals.pop().zip(vals.pop()).expect("length is < 2");
                            record.insert(key.coerce_into_string()?, val);
                        } else {
                            return Err(ShellError::IncorrectValue {
                                msg: format!(
                                    "expected inner list with two elements, but found {} element(s)",
                                    vals.len()
                                ),
                                val_span: span,
                                call_span: call.head,
                            });
                        }
                        expected_type = Some(ExpectedType::Pair);
                    }
                    Value::Nothing { .. } => {}
                    Value::Error { error, .. } => return Err(*error),
                    _ => {
                        return Err(ShellError::TypeMismatch {
                            err_message: format!(
                                "expected {}, found {} (while building record from list)",
                                match expected_type {
                                    Some(ExpectedType::Record) => "record",
                                    Some(ExpectedType::Pair) => "list with two elements",
                                    None => "record or list with two elements",
                                },
                                item.get_type(),
                            ),
                            span,
                        });
                    }
                }
            }
            Ok(Value::record(record, span).into_pipeline_data_with_metadata(metadata))
        }
        PipelineData::Value(Value::Record { .. }, _) => Ok(input),
        PipelineData::Value(Value::Error { error, .. }, _) => Err(*error),
        other => Err(ShellError::TypeMismatch {
            err_message: format!("Can't convert {} to record", other.get_type()),
            span,
        }),
    }
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
            "millisecond" => Value::int(date.timestamp_subsec_millis() as i64, span),
            "microsecond" => Value::int((date.nanosecond() / 1_000 % 1_000) as i64, span),
            "nanosecond" => Value::int((date.nanosecond() % 1_000) as i64, span),
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

        test_examples(IntoRecord {})
    }
}
