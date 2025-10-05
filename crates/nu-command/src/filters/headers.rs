use nu_engine::command_prelude::*;
use nu_protocol::Config;

#[derive(Clone)]
pub struct Headers;

impl Command for Headers {
    fn name(&self) -> &str {
        "headers"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::table(), Type::table())])
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Use the first row of the table as column names."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Sets the column names for a table created by `split column`",
                example: r#""a b c|1 2 3" | split row "|" | split column " " | headers"#,
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "a" => Value::test_string("1"),
                    "b" => Value::test_string("2"),
                    "c" => Value::test_string("3"),
                })])),
            },
            Example {
                description: "Columns which don't have data in their first row are removed",
                example: r#""a b c|1 2 3|1 2 3 4" | split row "|" | split column " " | headers"#,
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "a" => Value::test_string("1"),
                        "b" => Value::test_string("2"),
                        "c" => Value::test_string("3"),
                    }),
                    Value::test_record(record! {
                        "a" => Value::test_string("1"),
                        "b" => Value::test_string("2"),
                        "c" => Value::test_string("3"),
                    }),
                ])),
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
        let config = engine_state.get_config();
        let metadata = input.metadata();
        let span = input.span().unwrap_or(call.head);
        let value = input.into_value(span)?;
        let Value::List { vals: table, .. } = value else {
            return Err(ShellError::TypeMismatch {
                err_message: "not a table".to_string(),
                span,
            });
        };

        let (old_headers, new_headers) = extract_headers(&table, span, config)?;
        let value = replace_headers(table, span, &old_headers, &new_headers)?;

        Ok(value.into_pipeline_data_with_metadata(metadata))
    }
}

fn extract_headers(
    table: &[Value],
    span: Span,
    config: &Config,
) -> Result<(Vec<String>, Vec<String>), ShellError> {
    table
        .first()
        .ok_or_else(|| ShellError::GenericError {
            error: "Found empty list".into(),
            msg: "unable to extract headers".into(),
            span: Some(span),
            help: None,
            inner: vec![],
        })
        .and_then(Value::as_record)
        .and_then(|record| {
            for v in record.values() {
                if !is_valid_header(v) {
                    return Err(ShellError::TypeMismatch {
                        err_message: "needs compatible type: Null, String, Bool, Float, Int"
                            .to_string(),
                        span: v.span(),
                    });
                }
            }

            let old_headers = record.columns().cloned().collect();
            let new_headers = record
                .values()
                .enumerate()
                .map(|(idx, value)| {
                    let col = value.to_expanded_string("", config);
                    if col.is_empty() {
                        format!("column{idx}")
                    } else {
                        col
                    }
                })
                .collect();

            Ok((old_headers, new_headers))
        })
}

fn is_valid_header(value: &Value) -> bool {
    matches!(
        value,
        Value::Nothing { .. }
            | Value::String { val: _, .. }
            | Value::Bool { val: _, .. }
            | Value::Float { val: _, .. }
            | Value::Int { val: _, .. }
    )
}

fn replace_headers(
    rows: Vec<Value>,
    span: Span,
    old_headers: &[String],
    new_headers: &[String],
) -> Result<Value, ShellError> {
    rows.into_iter()
        .skip(1)
        .map(|value| {
            let span = value.span();
            if let Value::Record { val: record, .. } = value {
                Ok(Value::record(
                    record
                        .into_owned()
                        .into_iter()
                        .filter_map(|(col, val)| {
                            old_headers
                                .iter()
                                .position(|c| c == &col)
                                .map(|i| (new_headers[i].clone(), val))
                        })
                        .collect(),
                    span,
                ))
            } else {
                Err(ShellError::CantConvert {
                    to_type: "record".into(),
                    from_type: value.get_type().to_string(),
                    span,
                    help: None,
                })
            }
        })
        .collect::<Result<_, _>>()
        .map(|rows| Value::list(rows, span))
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
