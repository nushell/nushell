use nu_engine::command_prelude::*;

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
            .switch("strict", "follow the json specification exactly", Some('s'))
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
            Example {
                example: r#"'{ "a": 1, "b": 2 }' | from json -s"#,
                description: "Parse json strictly which will error on comments and trailing commas",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(1),
                    "b" => Value::test_int(2),
                })),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        let (string_input, span, metadata) = input.collect_string_strict(span)?;

        if string_input.is_empty() {
            return Ok(PipelineData::new_with_metadata(metadata, span));
        }

        let strict = call.has_flag(engine_state, stack, "strict")?;

        // TODO: turn this into a structured underline of the nu_json error
        if call.has_flag(engine_state, stack, "objects")? {
            let lines = string_input.lines().filter(|line| !line.trim().is_empty());

            let converted_lines: Vec<_> = if strict {
                lines
                    .map(|line| {
                        convert_string_to_value_strict(line, span)
                            .unwrap_or_else(|err| Value::error(err, span))
                    })
                    .collect()
            } else {
                lines
                    .map(|line| {
                        convert_string_to_value(line, span)
                            .unwrap_or_else(|err| Value::error(err, span))
                    })
                    .collect()
            };

            Ok(converted_lines
                .into_pipeline_data_with_metadata(metadata, engine_state.ctrlc.clone()))
        } else if strict {
            Ok(convert_string_to_value_strict(&string_input, span)?
                .into_pipeline_data_with_metadata(metadata))
        } else {
            Ok(convert_string_to_value(&string_input, span)?
                .into_pipeline_data_with_metadata(metadata))
        }
    }
}

fn convert_nujson_to_value(value: nu_json::Value, span: Span) -> Value {
    match value {
        nu_json::Value::Array(array) => Value::list(
            array
                .into_iter()
                .map(|x| convert_nujson_to_value(x, span))
                .collect(),
            span,
        ),
        nu_json::Value::Bool(b) => Value::bool(b, span),
        nu_json::Value::F64(f) => Value::float(f, span),
        nu_json::Value::I64(i) => Value::int(i, span),
        nu_json::Value::Null => Value::nothing(span),
        nu_json::Value::Object(k) => Value::record(
            k.into_iter()
                .map(|(k, v)| (k, convert_nujson_to_value(v, span)))
                .collect(),
            span,
        ),
        nu_json::Value::U64(u) => {
            if u > i64::MAX as u64 {
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
                Value::int(u as i64, span)
            }
        }
        nu_json::Value::String(s) => Value::string(s, span),
    }
}

// Converts row+column to a Span, assuming bytes (1-based rows)
fn convert_row_column_to_span(row: usize, col: usize, contents: &str) -> Span {
    let mut cur_row = 1;
    let mut cur_col = 1;

    for (offset, curr_byte) in contents.bytes().enumerate() {
        if curr_byte == b'\n' {
            cur_row += 1;
            cur_col = 1;
        }
        if cur_row >= row && cur_col >= col {
            return Span::new(offset, offset);
        } else {
            cur_col += 1;
        }
    }

    Span::new(contents.len(), contents.len())
}

fn convert_string_to_value(string_input: &str, span: Span) -> Result<Value, ShellError> {
    match nu_json::from_str(string_input) {
        Ok(value) => Ok(convert_nujson_to_value(value, span)),

        Err(x) => match x {
            nu_json::Error::Syntax(_, row, col) => {
                let label = x.to_string();
                let label_span = convert_row_column_to_span(row, col, string_input);
                Err(ShellError::GenericError {
                    error: "Error while parsing JSON text".into(),
                    msg: "error parsing JSON text".into(),
                    span: Some(span),
                    help: None,
                    inner: vec![ShellError::OutsideSpannedLabeledError {
                        src: string_input.into(),
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

fn convert_string_to_value_strict(string_input: &str, span: Span) -> Result<Value, ShellError> {
    match serde_json::from_str(string_input) {
        Ok(value) => Ok(convert_nujson_to_value(value, span)),
        Err(err) => Err(if err.is_syntax() {
            let label = err.to_string();
            let label_span = convert_row_column_to_span(err.line(), err.column(), string_input);
            ShellError::GenericError {
                error: "Error while parsing JSON text".into(),
                msg: "error parsing JSON text".into(),
                span: Some(span),
                help: None,
                inner: vec![ShellError::OutsideSpannedLabeledError {
                    src: string_input.into(),
                    error: "Error while parsing JSON text".into(),
                    msg: label,
                    span: label_span,
                }],
            }
        } else {
            ShellError::CantConvert {
                to_type: format!("structured json data ({err})"),
                from_type: "string".into(),
                span,
                help: None,
            }
        }),
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
