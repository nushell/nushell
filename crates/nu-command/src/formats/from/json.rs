use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Value};

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
        Signature::build("from json").switch(
            "objects",
            "treat each line as a separate value",
            Some('o'),
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "'{ a:1 }' | from json",
                description: "Converts json formatted string to table",
                result: Some(Value::Record {
                    cols: vec!["a".to_string()],
                    vals: vec![Value::Int {
                        val: 1,
                        span: Span::unknown(),
                    }],
                    span: Span::unknown(),
                }),
            },
            Example {
                example: "'{ a:1, b: [1, 2] }' | from json",
                description: "Converts json formatted string to table",
                result: Some(Value::Record {
                    cols: vec!["a".to_string(), "b".to_string()],
                    vals: vec![
                        Value::Int {
                            val: 1,
                            span: Span::unknown(),
                        },
                        Value::List {
                            vals: vec![
                                Value::Int {
                                    val: 1,
                                    span: Span::unknown(),
                                },
                                Value::Int {
                                    val: 2,
                                    span: Span::unknown(),
                                },
                            ],
                            span: Span::unknown(),
                        },
                    ],
                    span: Span::unknown(),
                }),
            },
        ]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        let span = call.head;
        let mut string_input = input.collect_string();
        string_input.push('\n');

        // TODO: turn this into a structured underline of the nu_json error
        if call.has_flag("objects") {
            #[allow(clippy::needless_collect)]
            let lines: Vec<String> = string_input.lines().map(|x| x.to_string()).collect();
            Ok(lines
                .into_iter()
                .map(move |mut x| {
                    x.push('\n');
                    match convert_string_to_value(x, span) {
                        Ok(v) => v,
                        Err(error) => Value::Error { error },
                    }
                })
                .into_pipeline_data())
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
                    error: ShellError::CantConvert("i64 sized integer".into(), span),
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

fn convert_string_to_value(string_input: String, span: Span) -> Result<Value, ShellError> {
    let result: Result<nu_json::Value, nu_json::Error> = nu_json::from_str(&string_input);
    match result {
        Ok(value) => Ok(convert_nujson_to_value(&value, span)),

        Err(_x) => Err(ShellError::CantConvert(
            "structured data from json".into(),
            span,
        )),
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
