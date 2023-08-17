use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Config, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span,
    SpannedValue, Type,
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
                description: "Sets the column names for a table created by `split column`",
                example: r#""a b c|1 2 3" | split row "|" | split column " " | headers"#,
                result: Some(SpannedValue::List {
                    vals: vec![SpannedValue::Record {
                        cols: columns.clone(),
                        vals: vec![
                            SpannedValue::test_string("1"),
                            SpannedValue::test_string("2"),
                            SpannedValue::test_string("3"),
                        ],
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Columns which don't have data in their first row are removed",
                example: r#""a b c|1 2 3|1 2 3 4" | split row "|" | split column " " | headers"#,
                result: Some(SpannedValue::List {
                    vals: vec![
                        SpannedValue::Record {
                            cols: columns.clone(),
                            vals: vec![
                                SpannedValue::test_string("1"),
                                SpannedValue::test_string("2"),
                                SpannedValue::test_string("3"),
                            ],
                            span: Span::test_data(),
                        },
                        SpannedValue::Record {
                            cols: columns,
                            vals: vec![
                                SpannedValue::test_string("1"),
                                SpannedValue::test_string("2"),
                                SpannedValue::test_string("3"),
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
    ) -> Result<PipelineData, ShellError> {
        let config = engine_state.get_config();
        let metadata = input.metadata();
        let value = input.into_value(call.head);
        let (old_headers, new_headers) = extract_headers(&value, config)?;
        let new_headers = replace_headers(value, &old_headers, &new_headers)?;

        Ok(new_headers.into_pipeline_data().set_metadata(metadata))
    }
}

fn replace_headers(
    value: SpannedValue,
    old_headers: &[String],
    new_headers: &[String],
) -> Result<SpannedValue, ShellError> {
    match value {
        SpannedValue::Record { cols, vals, span } => {
            let (cols, vals) = cols
                .into_iter()
                .zip(vals)
                .filter_map(|(col, val)| {
                    old_headers
                        .iter()
                        .position(|c| c == &col)
                        .map(|i| (new_headers[i].clone(), val))
                })
                .unzip();

            Ok(SpannedValue::Record { cols, vals, span })
        }
        SpannedValue::List { vals, span } => {
            let vals = vals
                .into_iter()
                .skip(1)
                .map(|value| replace_headers(value, old_headers, new_headers))
                .collect::<Result<Vec<SpannedValue>, ShellError>>()?;

            Ok(SpannedValue::List { vals, span })
        }
        _ => Err(ShellError::TypeMismatch {
            err_message: "record".to_string(),
            span: value.span()?,
        }),
    }
}

fn is_valid_header(value: &SpannedValue) -> bool {
    matches!(
        value,
        SpannedValue::Nothing { span: _ }
            | SpannedValue::String { val: _, span: _ }
            | SpannedValue::Bool { val: _, span: _ }
            | SpannedValue::Float { val: _, span: _ }
            | SpannedValue::Int { val: _, span: _ }
    )
}

fn extract_headers(
    value: &SpannedValue,
    config: &Config,
) -> Result<(Vec<String>, Vec<String>), ShellError> {
    match value {
        SpannedValue::Record { cols, vals, .. } => {
            for v in vals {
                if !is_valid_header(v) {
                    return Err(ShellError::TypeMismatch {
                        err_message: "needs compatible type: Null, String, Bool, Float, Int"
                            .to_string(),
                        span: v.span()?,
                    });
                }
            }

            let old_headers = cols.to_vec();
            let new_headers = vals
                .iter()
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
        SpannedValue::List { vals, span } => vals
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
        _ => Err(ShellError::TypeMismatch {
            err_message: "record".to_string(),
            span: value.span()?,
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
