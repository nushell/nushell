use crate::semver::value::SemverValue;
use chrono::{DateTime, Datelike, FixedOffset, Timelike};
use nu_engine::command_prelude::*;
use nu_protocol::{DurationMaxUnit, format_duration_as_timeperiod};

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
        "Convert value to a record."
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
                description: "Convert from one row table to record.",
                example: "[[value]; [false]] | into record",
                result: Some(Value::test_record(record! {
                    "value" => Value::test_bool(false),
                })),
            },
            Example {
                description: "Convert from list of records to record.",
                example: "[{foo: bar} {baz: quux}] | into record",
                result: Some(Value::test_record(record! {
                    "foo" => Value::test_string("bar"),
                    "baz" => Value::test_string("quux"),
                })),
            },
            Example {
                description: "Convert from list of pairs into record.",
                example: "[[foo bar] [baz quux]] | into record",
                result: Some(Value::test_record(record! {
                    "foo" => Value::test_string("bar"),
                    "baz" => Value::test_string("quux"),
                })),
            },
            Example {
                description: "convert duration to record (weeks max).",
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
                description: "convert record to record.",
                example: "{a: 1, b: 2} | into record",
                result: Some(Value::test_record(record! {
                    "a" =>  Value::test_int(1),
                    "b" =>  Value::test_int(2),
                })),
            },
            Example {
                description: "convert date to record.",
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
                description: "convert date components to table columns.",
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
        PipelineData::Value(Value::Custom { val, .. }, _) => {
            if let Some(semver) = val.as_any().downcast_ref::<SemverValue>() {
                Ok(parse_semver_into_record(semver, span).into_pipeline_data())
            } else {
                Err(ShellError::TypeMismatch {
                    err_message: format!("Can't convert {} to record", val.type_name()),
                    span,
                })
            }
        }
        PipelineData::Value(Value::List { .. }, _) | PipelineData::ListStream(..) => {
            let mut input = input;
            let mut record = Record::new();
            let metadata = input.take_metadata();

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
                    Value::List { vals, .. }
                        if matches!(expected_type, None | Some(ExpectedType::Pair)) =>
                    {
                        if vals.len() == 2 {
                            let mut vals = vals.into_owned();
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
    let (sign, periods) = format_duration_as_timeperiod(duration, DurationMaxUnit::default());

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
mod tests {
    use super::*;
    use crate::semver::value::SemverValue;

    fn create_semver_value(version: &str) -> Value {
        let semver = SemverValue::new(semver::Version::parse(version).unwrap());
        Value::custom(Box::new(semver), Span::test_data())
    }

    #[test]
    fn test_parse_semver_into_record_basic() {
        let semver_val = SemverValue::new(semver::Version::parse("1.2.3").unwrap());
        let result = parse_semver_into_record(&semver_val, Span::test_data());

        match result {
            Value::Record { val, .. } => {
                assert_eq!(val.get("major").unwrap().as_int().unwrap(), 1);
                assert_eq!(val.get("minor").unwrap().as_int().unwrap(), 2);
                assert_eq!(val.get("patch").unwrap().as_int().unwrap(), 3);
                assert_eq!(val.get("pre").unwrap().as_str().unwrap(), "");
                assert_eq!(val.get("build").unwrap().as_str().unwrap(), "");

                let pre_identifiers = val.get("pre_identifiers").unwrap().as_list().unwrap();
                assert_eq!(pre_identifiers.len(), 0);

                let build_identifiers = val.get("build_identifiers").unwrap().as_list().unwrap();
                assert_eq!(build_identifiers.len(), 0);
            }
            _ => panic!("Expected Record value"),
        }
    }

    #[test]
    fn test_parse_semver_into_record_with_prerelease() {
        let semver_val = SemverValue::new(semver::Version::parse("1.2.3-alpha.1").unwrap());
        let result = parse_semver_into_record(&semver_val, Span::test_data());

        match result {
            Value::Record { val, .. } => {
                assert_eq!(val.get("pre").unwrap().as_str().unwrap(), "alpha.1");

                let pre_identifiers = val.get("pre_identifiers").unwrap().as_list().unwrap();
                assert_eq!(pre_identifiers.len(), 2);
                assert_eq!(pre_identifiers[0].as_str().unwrap(), "alpha");
                assert_eq!(pre_identifiers[1].as_int().unwrap(), 1);
            }
            _ => panic!("Expected Record value"),
        }
    }

    #[test]
    fn test_parse_semver_into_record_with_build() {
        let semver_val = SemverValue::new(semver::Version::parse("1.2.3+build.2").unwrap());
        let result = parse_semver_into_record(&semver_val, Span::test_data());

        match result {
            Value::Record { val, .. } => {
                assert_eq!(val.get("build").unwrap().as_str().unwrap(), "build.2");

                let build_identifiers = val.get("build_identifiers").unwrap().as_list().unwrap();
                assert_eq!(build_identifiers.len(), 2);
                assert_eq!(build_identifiers[0].as_str().unwrap(), "build");
                assert_eq!(build_identifiers[1].as_int().unwrap(), 2);
            }
            _ => panic!("Expected Record value"),
        }
    }

    #[test]
    fn test_parse_semver_into_record_with_both() {
        let semver_val = SemverValue::new(semver::Version::parse("1.2.3-alpha.1+build.2").unwrap());
        let result = parse_semver_into_record(&semver_val, Span::test_data());

        match result {
            Value::Record { val, .. } => {
                assert_eq!(val.get("major").unwrap().as_int().unwrap(), 1);
                assert_eq!(val.get("minor").unwrap().as_int().unwrap(), 2);
                assert_eq!(val.get("patch").unwrap().as_int().unwrap(), 3);
                assert_eq!(val.get("pre").unwrap().as_str().unwrap(), "alpha.1");
                assert_eq!(val.get("build").unwrap().as_str().unwrap(), "build.2");

                let pre_identifiers = val.get("pre_identifiers").unwrap().as_list().unwrap();
                assert_eq!(pre_identifiers.len(), 2);

                let build_identifiers = val.get("build_identifiers").unwrap().as_list().unwrap();
                assert_eq!(build_identifiers.len(), 2);
            }
            _ => panic!("Expected Record value"),
        }
    }

    #[test]
    fn test_into_record_with_semver() {
        let semver_val = create_semver_value("1.2.3");
        let semver_ref = match &semver_val {
            Value::Custom { val, .. } => val.as_any().downcast_ref::<SemverValue>().unwrap(),
            _ => panic!("Expected Custom value"),
        };
        let result = parse_semver_into_record(semver_ref, Span::test_data());

        match result {
            Value::Record { val, .. } => {
                assert_eq!(val.get("major").unwrap().as_int().unwrap(), 1);
                assert_eq!(val.get("minor").unwrap().as_int().unwrap(), 2);
                assert_eq!(val.get("patch").unwrap().as_int().unwrap(), 3);
            }
            _ => panic!("Expected Record value"),
        }
    }
}

fn parse_semver_into_record(semver: &SemverValue, span: Span) -> Value {
    let version = &semver.version;

    let pre_identifiers: Vec<Value> = if version.pre.is_empty() {
        Vec::new()
    } else {
        version
            .pre
            .split('.')
            .map(|id| {
                if let Ok(num) = id.parse::<i64>() {
                    Value::int(num, span)
                } else {
                    Value::string(id.to_string(), span)
                }
            })
            .collect()
    };

    let build_identifiers: Vec<Value> = if version.build.is_empty() {
        Vec::new()
    } else {
        version
            .build
            .split('.')
            .map(|id| {
                if let Ok(num) = id.parse::<i64>() {
                    Value::int(num, span)
                } else {
                    Value::string(id.to_string(), span)
                }
            })
            .collect()
    };

    Value::record(
        record! {
            "major" => Value::int(version.major as i64, span),
            "minor" => Value::int(version.minor as i64, span),
            "patch" => Value::int(version.patch as i64, span),
            "pre" => Value::string(version.pre.to_string(), span),
            "build" => Value::string(version.build.to_string(), span),
            "pre_identifiers" => Value::list(pre_identifiers, span),
            "build_identifiers" => Value::list(build_identifiers, span),
        },
        span,
    )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(IntoRecord)
    }
}
