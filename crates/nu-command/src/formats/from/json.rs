use std::io::{BufRead, Cursor};

use nu_engine::command_prelude::*;
use nu_protocol::{ListStream, PipelineMetadata, Signals};

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
            Example {
                example: r#"'{ "a": 1 }
{ "b": 2 }' | from json --objects"#,
                description: "Parse a stream of line-delimited JSON values",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {"a" => Value::test_int(1)}),
                    Value::test_record(record! {"b" => Value::test_int(2)}),
                ])),
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

        let strict = call.has_flag(engine_state, stack, "strict")?;

        // TODO: turn this into a structured underline of the nu_json error
        if call.has_flag(engine_state, stack, "objects")? {
            // Return a stream of JSON values, one for each non-empty line
            match input {
                PipelineData::Value(Value::String { val, .. }, metadata) => {
                    Ok(PipelineData::ListStream(
                        read_json_lines(
                            Cursor::new(val),
                            span,
                            strict,
                            engine_state.signals().clone(),
                        ),
                        update_metadata(metadata),
                    ))
                }
                PipelineData::ByteStream(stream, metadata)
                    if stream.type_() != ByteStreamType::Binary =>
                {
                    if let Some(reader) = stream.reader() {
                        Ok(PipelineData::ListStream(
                            read_json_lines(reader, span, strict, Signals::empty()),
                            update_metadata(metadata),
                        ))
                    } else {
                        Ok(PipelineData::Empty)
                    }
                }
                _ => Err(ShellError::OnlySupportsThisInputType {
                    exp_input_type: "string".into(),
                    wrong_type: input.get_type().to_string(),
                    dst_span: call.head,
                    src_span: input.span().unwrap_or(call.head),
                }),
            }
        } else {
            // Return a single JSON value
            let (string_input, span, metadata) = input.collect_string_strict(span)?;

            if string_input.is_empty() {
                return Ok(Value::nothing(span).into_pipeline_data());
            }

            if strict {
                Ok(convert_string_to_value_strict(&string_input, span)?
                    .into_pipeline_data_with_metadata(update_metadata(metadata)))
            } else {
                Ok(convert_string_to_value(&string_input, span)?
                    .into_pipeline_data_with_metadata(update_metadata(metadata)))
            }
        }
    }
}

/// Create a stream of values from a reader that produces line-delimited JSON
fn read_json_lines(
    input: impl BufRead + Send + 'static,
    span: Span,
    strict: bool,
    signals: Signals,
) -> ListStream {
    let iter = input
        .lines()
        .filter(|line| line.as_ref().is_ok_and(|line| !line.trim().is_empty()) || line.is_err())
        .map(move |line| {
            let line = line.err_span(span)?;
            if strict {
                convert_string_to_value_strict(&line, span)
            } else {
                convert_string_to_value(&line, span)
            }
        })
        .map(move |result| result.unwrap_or_else(|err| Value::error(err, span)));

    ListStream::new(iter, span, signals)
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

fn update_metadata(metadata: Option<PipelineMetadata>) -> Option<PipelineMetadata> {
    metadata
        .map(|md| md.with_content_type(Some("application/json".into())))
        .or_else(|| {
            Some(PipelineMetadata::default().with_content_type(Some("application/json".into())))
        })
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
