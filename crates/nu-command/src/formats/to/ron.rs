use nu_engine::command_prelude::*;
use ron::Value as RonValue;

#[derive(Clone)]
pub struct ToRon;

impl Command for ToRon {
    fn name(&self) -> &str {
        "to ron"
    }

    fn signature(&self) -> Signature {
        Signature::build("to ron")
            .input_output_types(vec![(Type::Any, Type::String)])
            .switch("raw", "Do not pretty-print the output", Some('r'))
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Emit text in Rusty Object Notation (RON) format."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let raw = call.has_flag(engine_state, stack, "raw")?;
        let value = input.into_value(call.head)?;
        let span = value.span();

        let ron_value = convert_value_to_ron(&value, span)?;

        let output_string = if raw {
            ron::ser::to_string(&ron_value)
        } else {
            ron::ser::to_string_pretty(
                &ron_value,
                ron::ser::PrettyConfig::default().indentor("    ".to_string()),
            )
        }
        .map_err(|err| ShellError::CantConvert {
            to_type: "RON string".into(),
            from_type: "structured data".into(),
            span,
            help: Some(err.to_string()),
        })?;

        Ok(Value::string(output_string, span).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            example: "{a: 1, b: \"hello\"} | to ron",
            description: "Converts a Nushell record to a RON string.",
            result: Some(Value::test_string(
                "{\n    \"a\": 1,\n    \"b\": \"hello\",\n}",
            )),
        }]
    }
}

fn convert_value_to_ron(value: &Value, span: Span) -> Result<RonValue, ShellError> {
    match value {
        Value::Bool { val, .. } => Ok(RonValue::Bool(*val)),
        Value::Int { val, .. } => Ok(RonValue::Number(ron::Number::from(*val))),
        Value::Float { val, .. } => Ok(RonValue::Number(ron::Number::from(*val))),
        Value::String { val, .. } => Ok(RonValue::String(val.clone())),
        Value::Nothing { .. } => Ok(RonValue::Unit),
        Value::List { vals, .. } => {
            let seq = vals
                .iter()
                .map(|v| convert_value_to_ron(v, span))
                .collect::<Result<Vec<RonValue>, ShellError>>()?;
            Ok(RonValue::Seq(seq))
        }
        Value::Record { val, .. } => {
            let mut map = ron::Map::new();
            for (k, v) in val.iter() {
                map.insert(RonValue::String(k.clone()), convert_value_to_ron(v, span)?);
            }
            Ok(RonValue::Map(map))
        }
        Value::Filesize { val, .. } => Ok(RonValue::Number(ron::Number::from(val.get()))),
        Value::Date { val, .. } => Ok(RonValue::String(val.to_string())),
        Value::Duration { val, .. } => Ok(RonValue::Number(ron::Number::from(val.to_owned()))),
        Value::Range { val, .. } => Ok(RonValue::String(val.to_string())),
        Value::CellPath { val, .. } => Ok(RonValue::String(val.to_string())),
        // It doesn't appear I can implement support for closures easily, I'll leave
        // that for a later version, but it doesn't seem most other file formats do either
        Value::Binary {
            val, internal_span, ..
        } => convert_value_to_ron(
            &<std::vec::Vec<u8> as Clone>::clone(&val).into_value(span),
            *internal_span,
        ),
        _ => Err(ShellError::CantConvert {
            to_type: "structured ron data".into(),
            from_type: value.get_type().to_string(),
            span,
            help: Some("Unsupported type for RON serialization".into()),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(ToRon)
    }
}
