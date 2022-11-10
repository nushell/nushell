use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, ShellError,
    Signature, Span, Type, Value,
};

#[derive(Clone)]
pub struct FromJson;

impl Command for FromJson {
    fn name(&self) -> &str {
        "from json"
    }

    fn usage(&self) -> &str {
        "Convert from json to structured data"
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
                result: Some(Value::Record {
                    cols: vec!["a".to_string()],
                    vals: vec![Value::Int {
                        val: 1,
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: r#"'{ "a": 1, "b": [1, 2] }' | from json"#,
                description: "Converts json formatted string to table",
                result: Some(Value::Record {
                    cols: vec!["a".to_string(), "b".to_string()],
                    vals: vec![
                        Value::Int {
                            val: 1,
                            span: Span::test_data(),
                        },
                        Value::List {
                            vals: vec![
                                Value::Int {
                                    val: 1,
                                    span: Span::test_data(),
                                },
                                Value::Int {
                                    val: 2,
                                    span: Span::test_data(),
                                },
                            ],
                            span: Span::test_data(),
                        },
                    ],
                    span: Span::test_data(),
                }),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        let span = call.head;
        let config = engine_state.get_config();
        let string_input = input.collect_string("", config)?;

        if string_input.is_empty() {
            return Ok(PipelineData::new(span));
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
                            Err(error) => Some(Value::Error { error }),
                        }
                    }
                })
                .collect();
            Ok(converted_lines.into_pipeline_data(engine_state.ctrlc.clone()))
        } else {
            Ok(convert_string_to_value(string_input, span)?.into_pipeline_data())
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

            Value::List { vals: v, span }
        }
        nu_json::Value::Bool(b) => Value::Bool { val: *b, span },
        nu_json::Value::F64(f) => Value::Float { val: *f, span },
        nu_json::Value::I64(i) => Value::Int { val: *i, span },
        nu_json::Value::Null => Value::Nothing { span },
        nu_json::Value::Object(k) => {
            let mut cols = vec![];
            let mut vals = vec![];

            for item in k {
                cols.push(item.0.clone());
                vals.push(convert_nujson_to_value(item.1, span));
            }

            Value::Record { cols, vals, span }
        }
        nu_json::Value::U64(u) => {
            if *u > i64::MAX as u64 {
                Value::Error {
                    error: ShellError::CantConvert(
                        "i64 sized integer".into(),
                        "value larger than i64".into(),
                        span,
                        None,
                    ),
                }
            } else {
                Value::Int {
                    val: *u as i64,
                    span,
                }
            }
        }
        nu_json::Value::String(s) => Value::String {
            val: s.clone(),
            span,
        },
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
            return Span {
                start: offset,
                end: offset,
            };
        } else {
            cur_col += 1;
        }
    }

    Span {
        start: contents.len(),
        end: contents.len(),
    }
}

fn convert_string_to_value(string_input: String, span: Span) -> Result<Value, ShellError> {
    let result: Result<nu_json::Value, nu_json::Error> = nu_json::from_str(&string_input);
    match result {
        Ok(value) => Ok(convert_nujson_to_value(&value, span)),

        Err(x) => match x {
            nu_json::Error::Syntax(_, row, col) => {
                let label = x.to_string();
                let label_span = convert_row_column_to_span(row, col, &string_input);
                Err(ShellError::GenericError(
                    "Error while parsing JSON text".into(),
                    "error parsing JSON text".into(),
                    Some(span),
                    None,
                    vec![ShellError::OutsideSpannedLabeledError(
                        string_input,
                        "Error while parsing JSON text".into(),
                        label,
                        label_span,
                    )],
                ))
            }
            x => Err(ShellError::CantConvert(
                format!("structured json data ({})", x),
                "string".into(),
                span,
                None,
            )),
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
