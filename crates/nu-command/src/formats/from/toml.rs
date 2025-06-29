use nu_engine::command_prelude::*;
use toml::value::{Datetime, Offset};

#[derive(Clone)]
pub struct FromToml;

impl Command for FromToml {
    fn name(&self) -> &str {
        "from toml"
    }

    fn signature(&self) -> Signature {
        Signature::build("from toml")
            .input_output_types(vec![(Type::String, Type::record())])
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Parse text as .toml and create record."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        let (mut string_input, span, metadata) = input.collect_string_strict(span)?;
        string_input.push('\n');
        Ok(convert_string_to_value(string_input, span)?
            .into_pipeline_data_with_metadata(metadata.map(|md| md.with_content_type(None))))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "'a = 1' | from toml",
                description: "Converts toml formatted string to record",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(1),
                })),
            },
            Example {
                example: "'a = 1
b = [1, 2]' | from toml",
                description: "Converts toml formatted string to record",
                result: Some(Value::test_record(record! {
                    "a" =>  Value::test_int(1),
                    "b" =>  Value::test_list(vec![
                        Value::test_int(1),
                        Value::test_int(2)],),
                })),
            },
        ]
    }
}

fn convert_toml_datetime_to_value(dt: &Datetime, span: Span) -> Value {
    match &dt.clone() {
        toml::value::Datetime {
            date: Some(_),
            time: _,
            offset: _,
        } => (),
        _ => return Value::string(dt.to_string(), span),
    }

    let date = match dt.date {
        Some(date) => {
            chrono::NaiveDate::from_ymd_opt(date.year.into(), date.month.into(), date.day.into())
        }
        None => Some(chrono::NaiveDate::default()),
    };

    let time = match dt.time {
        Some(time) => chrono::NaiveTime::from_hms_nano_opt(
            time.hour.into(),
            time.minute.into(),
            time.second.into(),
            time.nanosecond,
        ),
        None => Some(chrono::NaiveTime::default()),
    };

    let tz = match dt.offset {
        Some(offset) => match offset {
            Offset::Z => chrono::FixedOffset::east_opt(0),
            Offset::Custom { minutes: min } => chrono::FixedOffset::east_opt(min as i32 * 60),
        },
        None => chrono::FixedOffset::east_opt(0),
    };

    let datetime = match (date, time, tz) {
        (Some(date), Some(time), Some(tz)) => chrono::NaiveDateTime::new(date, time)
            .and_local_timezone(tz)
            .earliest(),
        _ => None,
    };

    match datetime {
        Some(datetime) => Value::date(datetime, span),
        None => Value::string(dt.to_string(), span),
    }
}

fn convert_toml_to_value(value: &toml::Value, span: Span) -> Value {
    match value {
        toml::Value::Array(array) => {
            let v: Vec<Value> = array
                .iter()
                .map(|x| convert_toml_to_value(x, span))
                .collect();

            Value::list(v, span)
        }
        toml::Value::Boolean(b) => Value::bool(*b, span),
        toml::Value::Float(f) => Value::float(*f, span),
        toml::Value::Integer(i) => Value::int(*i, span),
        toml::Value::Table(k) => Value::record(
            k.iter()
                .map(|(k, v)| (k.clone(), convert_toml_to_value(v, span)))
                .collect(),
            span,
        ),
        toml::Value::String(s) => Value::string(s.clone(), span),
        toml::Value::Datetime(dt) => convert_toml_datetime_to_value(dt, span),
    }
}

pub fn convert_string_to_value(string_input: String, span: Span) -> Result<Value, ShellError> {
    let result: Result<toml::Value, toml::de::Error> = toml::from_str(&string_input);
    match result {
        Ok(value) => Ok(convert_toml_to_value(&value, span)),

        Err(err) => Err(ShellError::CantConvert {
            to_type: "structured toml data".into(),
            from_type: "string".into(),
            span,
            help: Some(err.to_string()),
        }),
    }
}

#[cfg(test)]
mod tests {
    use crate::Reject;
    use crate::{Metadata, MetadataSet};

    use super::*;
    use chrono::TimeZone;
    use nu_cmd_lang::eval_pipeline_without_terminal_expression;
    use toml::value::Datetime;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(FromToml {})
    }

    #[test]
    fn from_toml_creates_correct_date() {
        let toml_date = toml::Value::Datetime(Datetime {
            date: Option::from(toml::value::Date {
                year: 1980,
                month: 10,
                day: 12,
            }),
            time: Option::from(toml::value::Time {
                hour: 10,
                minute: 12,
                second: 44,
                nanosecond: 0,
            }),
            offset: Option::from(toml::value::Offset::Custom { minutes: 120 }),
        });

        let span = Span::test_data();
        let reference_date = Value::date(
            chrono::FixedOffset::east_opt(60 * 120)
                .unwrap()
                .with_ymd_and_hms(1980, 10, 12, 10, 12, 44)
                .unwrap(),
            Span::test_data(),
        );

        let result = convert_toml_to_value(&toml_date, span);

        //positive test (from toml returns a nushell date)
        assert_eq!(result, reference_date);
    }

    #[test]
    fn string_to_toml_value_passes() {
        let input_string = String::from(
            r#"
            command.build = "go build"

            [command.deploy]
            script = "./deploy.sh"
            "#,
        );

        let span = Span::test_data();

        let result = convert_string_to_value(input_string, span);

        assert!(result.is_ok());
    }

    #[test]
    fn string_to_toml_value_fails() {
        let input_string = String::from(
            r#"
            command.build =

            [command.deploy]
            script = "./deploy.sh"
            "#,
        );

        let span = Span::test_data();

        let result = convert_string_to_value(input_string, span);

        assert!(result.is_err());
    }

    #[test]
    fn convert_toml_datetime_to_value_date_time_offset() {
        let toml_date = Datetime {
            date: Option::from(toml::value::Date {
                year: 2000,
                month: 1,
                day: 1,
            }),
            time: Option::from(toml::value::Time {
                hour: 12,
                minute: 12,
                second: 12,
                nanosecond: 0,
            }),
            offset: Option::from(toml::value::Offset::Custom { minutes: 120 }),
        };

        let span = Span::test_data();
        let reference_date = Value::date(
            chrono::FixedOffset::east_opt(60 * 120)
                .unwrap()
                .with_ymd_and_hms(2000, 1, 1, 12, 12, 12)
                .unwrap(),
            span,
        );

        let result = convert_toml_datetime_to_value(&toml_date, span);

        assert_eq!(result, reference_date);
    }

    #[test]
    fn convert_toml_datetime_to_value_date_time() {
        let toml_date = Datetime {
            date: Option::from(toml::value::Date {
                year: 2000,
                month: 1,
                day: 1,
            }),
            time: Option::from(toml::value::Time {
                hour: 12,
                minute: 12,
                second: 12,
                nanosecond: 0,
            }),
            offset: None,
        };

        let span = Span::test_data();
        let reference_date = Value::date(
            chrono::FixedOffset::east_opt(0)
                .unwrap()
                .with_ymd_and_hms(2000, 1, 1, 12, 12, 12)
                .unwrap(),
            span,
        );

        let result = convert_toml_datetime_to_value(&toml_date, span);

        assert_eq!(result, reference_date);
    }

    #[test]
    fn convert_toml_datetime_to_value_date() {
        let toml_date = Datetime {
            date: Option::from(toml::value::Date {
                year: 2000,
                month: 1,
                day: 1,
            }),
            time: None,
            offset: None,
        };

        let span = Span::test_data();
        let reference_date = Value::date(
            chrono::FixedOffset::east_opt(0)
                .unwrap()
                .with_ymd_and_hms(2000, 1, 1, 0, 0, 0)
                .unwrap(),
            span,
        );

        let result = convert_toml_datetime_to_value(&toml_date, span);

        assert_eq!(result, reference_date);
    }

    #[test]
    fn convert_toml_datetime_to_value_only_time() {
        let toml_date = Datetime {
            date: None,
            time: Option::from(toml::value::Time {
                hour: 12,
                minute: 12,
                second: 12,
                nanosecond: 0,
            }),
            offset: None,
        };

        let span = Span::test_data();
        let reference_date = Value::string(toml_date.to_string(), span);

        let result = convert_toml_datetime_to_value(&toml_date, span);

        assert_eq!(result, reference_date);
    }

    #[test]
    fn test_content_type_metadata() {
        let mut engine_state = Box::new(EngineState::new());
        let delta = {
            let mut working_set = StateWorkingSet::new(&engine_state);

            working_set.add_decl(Box::new(FromToml {}));
            working_set.add_decl(Box::new(Metadata {}));
            working_set.add_decl(Box::new(MetadataSet {}));
            working_set.add_decl(Box::new(Reject {}));

            working_set.render()
        };

        engine_state
            .merge_delta(delta)
            .expect("Error merging delta");

        let cmd = r#""[a]\nb = 1\nc = 1" | metadata set --content-type 'text/x-toml' --datasource-ls | from toml | metadata | reject span | $in"#;
        let result = eval_pipeline_without_terminal_expression(
            cmd,
            std::env::temp_dir().as_ref(),
            &mut engine_state,
        );
        assert_eq!(
            Value::test_record(record!("source" => Value::test_string("ls"))),
            result.expect("There should be a result")
        )
    }
}
