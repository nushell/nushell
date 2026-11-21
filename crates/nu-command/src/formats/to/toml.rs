use crate::formats::{preserve_toml_document, read_toml_source_from_metadata};
use nu_engine::command_prelude::*;
use nu_protocol::{PipelineMetadata, ast::PathMember};

#[derive(Clone)]
pub struct ToToml;

impl Command for ToToml {
    fn name(&self) -> &str {
        "to toml"
    }

    fn signature(&self) -> Signature {
        Signature::build("to toml")
            .input_output_types(vec![(Type::record(), Type::String)])
            .switch(
                "serialize",
                "Serialize nushell types that cannot be deserialized.",
                Some('s'),
            )
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Convert record into .toml text."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Outputs an TOML string representing the contents of this record.",
            example: "{foo: 1 bar: 'qwe'} | to toml",
            result: Some(Value::test_string("foo = 1\nbar = \"qwe\"\n")),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let serialize_types = call.has_flag(engine_state, stack, "serialize")?;

        to_toml(engine_state, input, head, serialize_types)
    }
}

// Helper method to recursively convert nu_protocol::Value -> toml::Value
// This shouldn't be called at the top-level
pub(crate) fn nu_value_to_toml_value(
    engine_state: &EngineState,
    v: &Value,
    serialize_types: bool,
) -> Result<toml::Value, ShellError> {
    Ok(match &v {
        Value::Bool { val, .. } => toml::Value::Boolean(*val),
        Value::Int { val, .. } => toml::Value::Integer(*val),
        Value::Filesize { val, .. } => toml::Value::Integer(val.get()),
        Value::Duration { val, .. } => toml::Value::String(val.to_string()),
        Value::Date { val, .. } => toml::Value::Datetime(to_toml_datetime(val)),
        Value::Range { .. } => toml::Value::String("<Range>".to_string()),
        Value::Float { val, .. } => toml::Value::Float(*val),
        Value::String { val, .. } | Value::Glob { val, .. } => toml::Value::String(val.clone()),
        Value::Record { val, .. } => {
            let mut m = toml::map::Map::new();
            for (k, v) in &**val {
                m.insert(
                    k.clone(),
                    nu_value_to_toml_value(engine_state, v, serialize_types)?,
                );
            }
            toml::Value::Table(m)
        }
        Value::List { vals, .. } => {
            toml::Value::Array(toml_list(engine_state, vals, serialize_types)?)
        }
        Value::Closure { val, .. } => {
            if serialize_types {
                let block = engine_state.try_get_block(val.block_id);
                if let Some(span) = block.and_then(|block| block.span) {
                    let contents_bytes = engine_state.get_span_contents(span);
                    let contents_string = String::from_utf8_lossy(contents_bytes);
                    toml::Value::String(contents_string.to_string())
                } else {
                    toml::Value::String(format!(
                        "unable to retrieve block contents for toml block_id {}",
                        val.block_id.get()
                    ))
                }
            } else {
                toml::Value::String(format!("closure_{}", val.block_id.get()))
            }
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

fn toml_list(
    engine_state: &EngineState,
    input: &[Value],
    serialize_types: bool,
) -> Result<Vec<toml::Value>, ShellError> {
    let mut out = vec![];

    for value in input {
        out.push(nu_value_to_toml_value(
            engine_state,
            value,
            serialize_types,
        )?);
    }

    Ok(out)
}

fn toml_into_pipeline_data(
    toml_value: &toml::Value,
    value_type: Type,
    span: Span,
    metadata: Option<PipelineMetadata>,
) -> Result<PipelineData, ShellError> {
    let new_md = Some(
        metadata
            .unwrap_or_default()
            .with_content_type(Some("text/x-toml".into())),
    );

    match toml::to_string_pretty(&toml_value) {
        Ok(serde_toml_string) => {
            Ok(Value::string(serde_toml_string, span).into_pipeline_data_with_metadata(new_md))
        }
        _ => Ok(Value::error(
            ShellError::CantConvert {
                to_type: "TOML".into(),
                from_type: value_type.to_string(),
                span,
                help: None,
            },
            span,
        )
        .into_pipeline_data_with_metadata(new_md)),
    }
}

fn value_to_toml_value(
    engine_state: &EngineState,
    v: &Value,
    head: Span,
    serialize_types: bool,
) -> Result<toml::Value, ShellError> {
    match v {
        Value::Record { .. } | Value::Closure { .. } => {
            nu_value_to_toml_value(engine_state, v, serialize_types)
        }
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
    mut input: PipelineData,
    span: Span,
    serialize_types: bool,
) -> Result<PipelineData, ShellError> {
    let metadata = input.take_metadata();
    let value = input.into_value(span)?;

    if !serialize_types
        && let Some(original_source) = read_toml_source_from_metadata(metadata.as_ref())
        && let Value::Record { .. } = &value
        && let Ok(preserved) = preserve_toml_document(engine_state, &value, &original_source, span)
    {
        let new_md = Some(
            metadata
                .unwrap_or_default()
                .with_content_type(Some("text/x-toml".into())),
        );
        return Ok(Value::string(preserved, span).into_pipeline_data_with_metadata(new_md));
    }

    let toml_value = value_to_toml_value(engine_state, &value, span, serialize_types)?;
    match toml_value {
        toml::Value::Array(ref vec) => match vec[..] {
            [toml::Value::Table(_)] => toml_into_pipeline_data(
                vec.iter().next().expect("this should never trigger"),
                value.get_type(),
                span,
                metadata,
            ),
            _ => toml_into_pipeline_data(&toml_value, value.get_type(), span, metadata),
        },
        _ => toml_into_pipeline_data(&toml_value, value.get_type(), span, metadata),
    }
}

/// Convert a nushell date into a toml::Value datetime. The latter uses its own ad-hoc datetime
/// types, which makes this somewhat convoluted. TOML offsets are whole minutes, so an offset with
/// seconds is rounded and the wall clock moves with it.
fn to_toml_datetime(datetime: &nu_protocol::DateTime) -> toml::value::Datetime {
    let datetime = datetime.to_minute_offset();
    let civil = datetime.parts();
    let date = toml::value::Date {
        // TODO: figure out what to do with BC dates, because the toml crate doesn't support
        // them. Years before 1 CE count down from 1 BCE.
        year: if civil.year > 0 {
            civil.year as u16
        } else {
            (1 - civil.year) as u16
        },
        month: civil.month as u8,
        day: civil.day as u8,
    };

    let time = toml::value::Time {
        hour: civil.hour as u8,
        minute: civil.minute as u8,
        second: Some(civil.second as u8),
        nanosecond: Some(civil.nanosecond as u32),
    };

    let offset = toml::value::Offset::Custom {
        // A minute offset fits into i16 (that's more than 500 hours).
        minutes: (datetime.offset().seconds() / 60) as i16,
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

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(ToToml)
    }

    #[test]
    fn to_toml_creates_correct_date() {
        let engine_state = EngineState::new();
        let serialize_types = false;

        let test_date = Value::date(
            nu_protocol::DateTime::parse_rfc3339("1980-10-12T10:12:44+02:00").unwrap(),
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
                second: Some(44),
                nanosecond: Some(0),
            }),
            offset: Some(toml::value::Offset::Custom { minutes: 120 }),
        });

        let result = nu_value_to_toml_value(&engine_state, &test_date, serialize_types);

        assert!(result.is_ok_and(|res| res == reference_date));
    }

    #[test]
    fn second_60_and_offsets_with_seconds_round_trip() {
        use crate::formats::from::toml_datetime_to_value;
        // Second 60, a leap second, is the next minute's second 0.
        let leap = nu_protocol::DateTime::parse_rfc3339("2016-12-31T23:59:60.5Z").unwrap();
        // +00:17:30, Amsterdam's local mean time, rounds to +00:18 and moves the clock with it.
        let lmt = nu_protocol::DateTime::parse_rfc3339("1800-01-01T00:00:00Z")
            .unwrap()
            .with_offset(jiff::tz::Offset::from_seconds(1050).unwrap());
        assert_eq!(
            to_toml_datetime(&leap).to_string(),
            "2017-01-01T00:00:00.5+00:00"
        );
        assert_eq!(
            to_toml_datetime(&lmt).to_string(),
            "1800-01-01T00:18:00.0+00:18"
        );
        for date in [leap, lmt] {
            let back = toml_datetime_to_value(&to_toml_datetime(&date), Span::test_data());
            assert_eq!(back.as_date().ok(), Some(date));
        }
        // TOML text with second 60 reads the same way.
        let text: toml::value::Datetime = "2016-12-31T23:59:60.5Z".parse().unwrap();
        let read = toml_datetime_to_value(&text, Span::test_data());
        assert_eq!(read.as_date().ok(), Some(leap));
    }

    #[test]
    fn test_value_to_toml_value() {
        //
        // Positive Tests
        //

        let engine_state = EngineState::new();
        let serialize_types = false;

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
            serialize_types,
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
            serialize_types,
        )
        .expect_err("Expected non-valid toml (String) to cause error!");
        value_to_toml_value(
            &engine_state,
            &Value::list(vec![Value::test_string("1")], Span::test_data()),
            Span::test_data(),
            serialize_types,
        )
        .expect_err("Expected non-valid toml (Table) to cause error!");
    }
}
