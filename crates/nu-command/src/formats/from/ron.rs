use nu_engine::command_prelude::*;
use ron::Value as RonValue;

#[derive(Clone)]
pub struct FromRon;

impl Command for FromRon {
    fn name(&self) -> &str {
        "from ron"
    }

    fn signature(&self) -> Signature {
        Signature::build("from ron")
            .input_output_types(vec![(Type::String, Type::Any)])
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Parse text as Rusty Object Notation (RON) and create structured data."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        let (string_input, span, metadata) = input.collect_string_strict(span)?;

        Ok(convert_string_to_value(string_input, span)?
            .into_pipeline_data_with_metadata(metadata.map(|md| md.with_content_type(None))))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "'(a: 1, b: \"hello\")' | from ron",
                description: "Converts a RON map string to a Nushell record.",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(1),
                    "b" => Value::test_string("hello"),
                })),
            },
            Example {
                example: "'(1, 2, 3)' | from ron",
                description: "Converts a RON tuple string to a Nushell list.",
                result: Some(Value::test_list(vec![
                    Value::test_int(1),
                    Value::test_int(2),
                    Value::test_int(3),
                ])),
            },
        ]
    }
}

fn convert_ron_to_value(value: &RonValue, span: Span) -> Value {
    match value {
        RonValue::Bool(b) => Value::bool(*b, span),
        RonValue::Char(c) => Value::string(c.to_string(), span),
        RonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::int(i, span)
            } else if let Some(f) = n.as_f64() {
                Value::float(f, span)
            } else {
                Value::string(format!("{:?}", n), span)
            }
        }
        RonValue::String(s) => Value::string(s.clone(), span),
        RonValue::Option(opt) => match opt {
            Some(v) => convert_ron_to_value(v, span),
            None => Value::nothing(span),
        },
        RonValue::Seq(seq) => {
            let v: Vec<Value> = seq.iter().map(|x| convert_ron_to_value(x, span)).collect();
            Value::list(v, span)
        }
        RonValue::Map(map) => {
            let record_fields = map
                .iter()
                .map(|(k, v)| {
                    let key_str = match k {
                        RonValue::String(s) => s.clone(),
                        _ => format!("{:?}", k),
                    };
                    (key_str, convert_ron_to_value(v, span))
                })
                .collect();
            Value::record(record_fields, span)
        }
        RonValue::Unit => Value::nothing(span),
    }
}

pub(crate) fn convert_string_to_value(
    string_input: String,
    span: Span,
) -> Result<Value, ShellError> {
    let result: Result<RonValue, ron::error::SpannedError> = ron::from_str(&string_input);
    match result {
        Ok(value) => Ok(convert_ron_to_value(&value, span)),
        Err(err) => Err(ShellError::CantConvert {
            to_type: "structured ron data".into(),
            from_type: "string".into(),
            span,
            help: Some(err.to_string()),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(FromRon)
    }

    #[test]
    fn string_to_ron_value_passes() {
        let input_string = String::from(r#"(name: "test", count: 42)"#);
        let span = Span::test_data();
        let result = convert_string_to_value(input_string, span);
        assert!(result.is_ok());
    }
}
