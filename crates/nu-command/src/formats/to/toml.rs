use chrono::{DateTime, Datelike, FixedOffset, Timelike};
use nu_engine::command_prelude::*;
use nu_protocol::ast::PathMember;

#[derive(Clone)]
pub struct ToToml;

impl Command for ToToml {
    fn name(&self) -> &str {
        "to toml"
    }

    fn signature(&self) -> Signature {
        Signature::build("to toml")
            .input_output_types(vec![(Type::record(), Type::String)])
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "Convert record into .toml text."
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Outputs an TOML string representing the contents of this record",
            example: r#"{foo: 1 bar: 'qwe'} | to toml"#,
            result: Some(Value::test_string("foo = 1\nbar = \"qwe\"\n")),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        to_toml(engine_state, input, head)
    }
}

// Helper method to recursively convert nu_protocol::Value -> toml::Value
// This shouldn't be called at the top-level
fn helper(engine_state: &EngineState, v: &Value) -> Result<toml::Value, ShellError> {
    let span = v.span();
    Ok(match &v {
        Value::Bool { val, .. } => toml::Value::Boolean(*val),
        Value::Int { val, .. } => toml::Value::Integer(*val),
        Value::Filesize { val, .. } => toml::Value::Integer(*val),
        Value::Duration { val, .. } => toml::Value::String(val.to_string()),
        Value::Date { val, .. } => toml::Value::Datetime(to_toml_datetime(val)),
        Value::Range { .. } => toml::Value::String("<Range>".to_string()),
        Value::Float { val, .. } => toml::Value::Float(*val),
        Value::String { val, .. } | Value::Glob { val, .. } => toml::Value::String(val.clone()),
        Value::Record { val, .. } => {
            let mut m = toml::map::Map::new();
            for (k, v) in &**val {
                m.insert(k.clone(), helper(engine_state, v)?);
            }
            toml::Value::Table(m)
        }
        Value::List { vals, .. } => toml::Value::Array(toml_list(engine_state, vals)?),
        Value::Closure { .. } => {
            let code = engine_state.get_span_contents(span);
            let code = String::from_utf8_lossy(code).to_string();
            toml::Value::String(code)
        }
        Value::Nothing { .. } => toml::Value::String("<Nothing>".to_string()),
        Value::Error { error, .. } => return Err(*error.clone()),
        Value::Binary { val, .. } => toml::Value::Array(
            val.iter()
                .map(|x| toml::Value::Integer(*x as i64))
                .collect(),
        ),
        Value::CellPath { val, .. } => toml::Value::Array(
            val.members
                .iter()
                .map(|x| match &x {
                    PathMember::String { val, .. } => Ok(toml::Value::String(val.clone())),
                    PathMember::Int { val, .. } => Ok(toml::Value::Integer(*val as i64)),
                })
                .collect::<Result<Vec<toml::Value>, ShellError>>()?,
        ),
        Value::Custom { .. } => toml::Value::String("<Custom Value>".to_string()),
    })
}

fn toml_list(engine_state: &EngineState, input: &[Value]) -> Result<Vec<toml::Value>, ShellError> {
    let mut out = vec![];

    for value in input {
        out.push(helper(engine_state, value)?);
    }

    Ok(out)
}

fn toml_into_pipeline_data(
    toml_value: &toml::Value,
    value_type: Type,
    span: Span,
) -> Result<PipelineData, ShellError> {
    match toml::to_string_pretty(&toml_value) {
        Ok(serde_toml_string) => Ok(Value::string(serde_toml_string, span).into_pipeline_data()),
        _ => Ok(Value::error(
            ShellError::CantConvert {
                to_type: "TOML".into(),
                from_type: value_type.to_string(),
                span,
                help: None,
            },
            span,
        )
        .into_pipeline_data()),
    }
}

fn value_to_toml_value(
    engine_state: &EngineState,
    v: &Value,
    head: Span,
) -> Result<toml::Value, ShellError> {
    match v {
        Value::Record { .. } => helper(engine_state, v),
        // Propagate existing errors
        Value::Error { error, .. } => Err(*error.clone()),
        _ => Err(ShellError::UnsupportedInput {
            msg: format!("{:?} is not valid top-level TOML", v.get_type()),
            input: "value originates from here".into(),
            msg_span: head,
            input_span: v.span(),
        }),
    }
}

fn to_toml(
    engine_state: &EngineState,
    input: PipelineData,
    span: Span,
) -> Result<PipelineData, ShellError> {
    let value = input.into_value(span)?;

    let toml_value = value_to_toml_value(engine_state, &value, span)?;
    match toml_value {
        toml::Value::Array(ref vec) => match vec[..] {
            [toml::Value::Table(_)] => toml_into_pipeline_data(
                vec.iter().next().expect("this should never trigger"),
                value.get_type(),
                span,
            ),
            _ => toml_into_pipeline_data(&toml_value, value.get_type(), span),
        },
        _ => toml_into_pipeline_data(&toml_value, value.get_type(), span),
    }
}

/// Convert chrono datetime into a toml::Value datetime.  The latter uses its
/// own ad-hoc datetime types, which makes this somewhat convoluted.
fn to_toml_datetime(datetime: &DateTime<FixedOffset>) -> toml::value::Datetime {
    let date = toml::value::Date {
        // TODO: figure out what to do with BC dates, because the toml
        // crate doesn't support them.  Same for large years, which
        // don't fit in u16.
        year: datetime.year_ce().1 as u16,
        // Panic: this is safe, because chrono guarantees that the month
        // value will be between 1 and 12 and the day will be between 1
        // and 31
        month: datetime.month() as u8,
        day: datetime.day() as u8,
    };

    let time = toml::value::Time {
        // Panic: same as before, chorono guarantees that all of the following 3
        // methods return values less than 65'000
        hour: datetime.hour() as u8,
        minute: datetime.minute() as u8,
        second: datetime.second() as u8,
        nanosecond: datetime.nanosecond(),
    };

    let offset = toml::value::Offset::Custom {
        // Panic: minute timezone offset fits into i16 (that's more than
        // 1000 hours)
        minutes: (-datetime.timezone().utc_minus_local() / 60) as i16,
    };

    toml::value::Datetime {
        date: Some(date),
        time: Some(time),
        offset: Some(offset),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(ToToml {})
    }

    #[test]
    fn to_toml_creates_correct_date() {
        let engine_state = EngineState::new();

        let test_date = Value::date(
            chrono::FixedOffset::east_opt(60 * 120)
                .unwrap()
                .with_ymd_and_hms(1980, 10, 12, 10, 12, 44)
                .unwrap(),
            Span::test_data(),
        );

        let reference_date = toml::Value::Datetime(toml::value::Datetime {
            date: Some(toml::value::Date {
                year: 1980,
                month: 10,
                day: 12,
            }),
            time: Some(toml::value::Time {
                hour: 10,
                minute: 12,
                second: 44,
                nanosecond: 0,
            }),
            offset: Some(toml::value::Offset::Custom { minutes: 120 }),
        });

        let result = helper(&engine_state, &test_date);

        assert!(result.is_ok_and(|res| res == reference_date));
    }

    #[test]
    fn test_value_to_toml_value() {
        //
        // Positive Tests
        //

        let engine_state = EngineState::new();

        let mut m = indexmap::IndexMap::new();
        m.insert("rust".to_owned(), Value::test_string("editor"));
        m.insert("is".to_owned(), Value::nothing(Span::test_data()));
        m.insert(
            "features".to_owned(),
            Value::list(
                vec![Value::test_string("hello"), Value::test_string("array")],
                Span::test_data(),
            ),
        );
        let tv = value_to_toml_value(
            &engine_state,
            &Value::record(m.into_iter().collect(), Span::test_data()),
            Span::test_data(),
        )
        .expect("Expected Ok from valid TOML dictionary");
        assert_eq!(
            tv.get("features"),
            Some(&toml::Value::Array(vec![
                toml::Value::String("hello".to_owned()),
                toml::Value::String("array".to_owned())
            ]))
        );
        //
        // Negative Tests
        //
        value_to_toml_value(
            &engine_state,
            &Value::test_string("not_valid"),
            Span::test_data(),
        )
        .expect_err("Expected non-valid toml (String) to cause error!");
        value_to_toml_value(
            &engine_state,
            &Value::list(vec![Value::test_string("1")], Span::test_data()),
            Span::test_data(),
        )
        .expect_err("Expected non-valid toml (Table) to cause error!");
    }
}
