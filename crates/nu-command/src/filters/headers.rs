use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Config, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Type,
    Value,
};

#[derive(Clone)]
pub struct Headers;

impl Command for Headers {
    fn name(&self) -> &str {
        "headers"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (Type::Table(vec![]), Type::Table(vec![])),
                (
                    // Tables with missing values are List<Any>
                    Type::List(Box::new(Type::Any)),
                    Type::Table(vec![]),
                ),
            ])
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Use the first row of the table as column names."
    }

    fn examples(&self) -> Vec<Example> {
        let columns = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        vec![
            Example {
                description: "Returns headers from table",
                example: r#""a b c|1 2 3" | split row "|" | split column " " | headers"#,
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: columns.clone(),
                        vals: vec![
                            Value::test_string("1"),
                            Value::test_string("2"),
                            Value::test_string("3"),
                        ],
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Don't panic on rows with different headers",
                example: r#""a b c|1 2 3|1 2 3 4" | split row "|" | split column " " | headers"#,
                result: Some(Value::List {
                    vals: vec![
                        Value::Record {
                            cols: columns.clone(),
                            vals: vec![
                                Value::test_string("1"),
                                Value::test_string("2"),
                                Value::test_string("3"),
                            ],
                            span: Span::test_data(),
                        },
                        Value::Record {
                            cols: columns,
                            vals: vec![
                                Value::test_string("1"),
                                Value::test_string("2"),
                                Value::test_string("3"),
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
        let config = engine_state.get_config();
        let metadata = input.metadata();
        let value = input.into_value(call.head);
        let headers = extract_headers(&value, config)?;
        let new_headers = replace_headers(value, &headers)?;

        Ok(new_headers.into_pipeline_data().set_metadata(metadata))
    }
}

fn replace_headers(value: Value, headers: &[String]) -> Result<Value, ShellError> {
    match value {
        Value::Record { vals, span, .. } => {
            let vals = vals.into_iter().take(headers.len()).collect();
            Ok(Value::Record {
                cols: headers.to_owned(),
                vals,
                span,
            })
        }
        Value::List { vals, span } => {
            let vals = vals
                .into_iter()
                .skip(1)
                .map(|value| replace_headers(value, headers))
                .collect::<Result<Vec<Value>, ShellError>>()?;

            Ok(Value::List { vals, span })
        }
        _ => Err(ShellError::TypeMismatch(
            "record".to_string(),
            value.span()?,
        )),
    }
}

fn is_valid_header(value: &Value) -> bool {
    matches!(
        value,
        Value::Nothing { span: _ }
            | Value::String { val: _, span: _ }
            | Value::Bool { val: _, span: _ }
            | Value::Float { val: _, span: _ }
            | Value::Int { val: _, span: _ }
    )
}

fn extract_headers(value: &Value, config: &Config) -> Result<Vec<String>, ShellError> {
    match value {
        Value::Record { vals, .. } => {
            for v in vals {
                if !is_valid_header(v) {
                    return Err(ShellError::TypeMismatch(
                        "compatible type: Null, String, Bool, Float, Int".to_string(),
                        v.span()?,
                    ));
                }
            }

            Ok(vals
                .iter()
                .enumerate()
                .map(|(idx, value)| {
                    let col = value.into_string("", config);
                    if col.is_empty() {
                        format!("column{}", idx)
                    } else {
                        col
                    }
                })
                .collect::<Vec<String>>())
        }
        Value::List { vals, span } => vals
            .iter()
            .map(|value| extract_headers(value, config))
            .next()
            .ok_or_else(|| {
                ShellError::GenericError(
                    "Found empty list".to_string(),
                    "unable to extract headers".to_string(),
                    Some(*span),
                    None,
                    Vec::new(),
                )
            })?,
        _ => Err(ShellError::TypeMismatch(
            "record".to_string(),
            value.span()?,
        )),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Headers {})
    }
}
