use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    record, Category, Config, Example, IntoPipelineData, PipelineData, ShellError, Signature, Type,
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
        let value = input.into_value(call.head);
        let (old_headers, new_headers) = extract_headers(&value, config)?;
        let new_headers = replace_headers(value, &old_headers, &new_headers)?;

        Ok(new_headers.into_pipeline_data_with_metadata(metadata))
    }
}

fn replace_headers(
    value: Value,
    old_headers: &[String],
    new_headers: &[String],
) -> Result<Value, ShellError> {
    let span = value.span();
    match value {
        Value::Record { val, .. } => Ok(Value::record(
            val.into_iter()
                .filter_map(|(col, val)| {
                    old_headers
                        .iter()
                        .position(|c| c == &col)
                        .map(|i| (new_headers[i].clone(), val))
                })
                .collect(),
            span,
        )),
        Value::List { vals, .. } => {
            let vals = vals
                .into_iter()
                .skip(1)
                .map(|value| replace_headers(value, old_headers, new_headers))
                .collect::<Result<Vec<Value>, ShellError>>()?;

            Ok(Value::list(vals, span))
        }
        _ => Err(ShellError::TypeMismatch {
            err_message: "record".to_string(),
            span: value.span(),
        }),
    }
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

fn extract_headers(
    value: &Value,
    config: &Config,
) -> Result<(Vec<String>, Vec<String>), ShellError> {
    let span = value.span();
    match value {
        Value::Record { val: record, .. } => {
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
                    let col = value.into_string("", config);
                    if col.is_empty() {
                        format!("column{idx}")
                    } else {
                        col
                    }
                })
                .collect::<Vec<String>>();

            Ok((old_headers, new_headers))
        }
        Value::List { vals, .. } => vals
            .iter()
            .map(|value| extract_headers(value, config))
            .next()
            .ok_or_else(|| ShellError::GenericError {
                error: "Found empty list".into(),
                msg: "unable to extract headers".into(),
                span: Some(span),
                help: None,
                inner: vec![],
            })?,
        _ => Err(ShellError::TypeMismatch {
            err_message: "record".to_string(),
            span: value.span(),
        }),
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
