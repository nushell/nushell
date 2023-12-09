use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    record, Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData,
    ShellError, Signature, Span, Type, Value,
};

#[derive(Clone)]
pub struct FromJson;

impl Command for FromJson {
    fn name(&self) -> &str {
        "from json"
    }

    fn usage(&self) -> &str {
        "Convert from json to structured data."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("from json")
            .input_output_types(vec![(Type::String, Type::Any)])
            .switch("objects", "treat each line as a separate value", Some('o'))
            .category(Category::Formats)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: r#"'{ "a": 1 }' | from json"#,
                description: "Converts json formatted string to table",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(1),
                })),
            },
            Example {
                example: r#"'{ "a": 1, "b": [1, 2] }' | from json"#,
                description: "Converts json formatted string to table",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(1),
                    "b" => Value::test_list(vec![Value::test_int(1), Value::test_int(2)]),
                })),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        let (string_input, span, metadata) = input.collect_string_strict(span)?;

        if string_input.is_empty() {
            return Ok(PipelineData::new_with_metadata(metadata, span));
        }

        // TODO: turn this into a structured underline of the nu_json error
        if call.has_flag("objects") {
            let converted_lines: Vec<Value> = string_input
                .lines()
                .filter_map(move |x| {
                    if x.trim() == "" {
                        None
                    } else {
                        match convert_string_to_value(x.to_string(), span) {
                            Ok(v) => Some(v),
                            Err(error) => Some(Value::error(error, span)),
                        }
                    }
                })
                .collect();
            Ok(converted_lines
                .into_pipeline_data_with_metadata(metadata, engine_state.ctrlc.clone()))
        } else {
            Ok(convert_string_to_value(string_input, span)?
                .into_pipeline_data_with_metadata(metadata))
        }
    }
}

fn convert_nujson_to_value(value: &nu_json::Value, span: Span) -> Value {
    match value {
        nu_json::Value::Array(array) => {
            let v: Vec<Value> = array
                .iter()
                .map(|x| convert_nujson_to_value(x, span))
                .collect();

            Value::list(v, span)
        }
        nu_json::Value::Bool(b) => Value::bool(*b, span),
        nu_json::Value::F64(f) => Value::float(*f, span),
        nu_json::Value::I64(i) => Value::int(*i, span),
        nu_json::Value::Null => Value::nothing(span),
        nu_json::Value::Object(k) => Value::record(
            k.iter()
                .map(|(k, v)| (k.clone(), convert_nujson_to_value(v, span)))
                .collect(),
            span,
        ),
        nu_json::Value::U64(u) => {
            if *u > i64::MAX as u64 {
                Value::error(
                    ShellError::CantConvert {
                        to_type: "i64 sized integer".into(),
                        from_type: "value larger than i64".into(),
                        span,
                        help: None,
                    },
                    span,
                )
            } else {
                Value::int(*u as i64, span)
            }
        }
        nu_json::Value::String(s) => Value::string(s.clone(), span),
    }
}

// Converts row+column to a Span, assuming bytes (1-based rows)
fn convert_row_column_to_span(row: usize, col: usize, contents: &str) -> Span {
    let mut cur_row = 1;
    let mut cur_col = 0;

    for (offset, curr_byte) in contents.bytes().enumerate() {
        if curr_byte == b'\n' {
            cur_row += 1;
            cur_col = 0;
        }
        if cur_row >= row && cur_col >= col {
            return Span::new(offset, offset);
        } else {
            cur_col += 1;
        }
    }

    Span::new(contents.len(), contents.len())
}

fn convert_string_to_value(string_input: String, span: Span) -> Result<Value, ShellError> {
    let result: Result<nu_json::Value, nu_json::Error> = nu_json::from_str(&string_input);
    match result {
        Ok(value) => Ok(convert_nujson_to_value(&value, span)),

        Err(x) => match x {
            nu_json::Error::Syntax(_, row, col) => {
                let label = x.to_string();
                let label_span = convert_row_column_to_span(row, col, &string_input);
                Err(ShellError::GenericError {
                    error: "Error while parsing JSON text".into(),
                    msg: "error parsing JSON text".into(),
                    span: Some(span),
                    help: None,
                    inner: vec![ShellError::OutsideSpannedLabeledError {
                        src: string_input,
                        error: "Error while parsing JSON text".into(),
                        msg: label,
                        span: label_span,
                    }],
                })
            }
            x => Err(ShellError::CantConvert {
                to_type: format!("structured json data ({x})"),
                from_type: "string".into(),
                span,
                help: None,
            }),
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(FromJson {})
    }
}
