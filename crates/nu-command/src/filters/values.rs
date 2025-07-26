use indexmap::IndexMap;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct Values;

impl Command for Values {
    fn name(&self) -> &str {
        "values"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (Type::record(), Type::List(Box::new(Type::Any))),
                (Type::table(), Type::List(Box::new(Type::Any))),
            ])
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Given a record or table, produce a list of its columns' values."
    }

    fn extra_description(&self) -> &str {
        "This is a counterpart to `columns`, which produces a list of columns' names."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "{ mode:normal userid:31415 } | values",
                description: "Get the values from the record (produce a list)",
                result: Some(Value::list(
                    vec![Value::test_string("normal"), Value::test_int(31415)],
                    Span::test_data(),
                )),
            },
            Example {
                example: "{ f:250 g:191 c:128 d:1024 e:2000 a:16 b:32 } | values",
                description: "Values are ordered by the column order of the record",
                result: Some(Value::list(
                    vec![
                        Value::test_int(250),
                        Value::test_int(191),
                        Value::test_int(128),
                        Value::test_int(1024),
                        Value::test_int(2000),
                        Value::test_int(16),
                        Value::test_int(32),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                example: "[[name meaning]; [ls list] [mv move] [cd 'change directory']] | values",
                description: "Get the values from the table (produce a list of lists)",
                result: Some(Value::list(
                    vec![
                        Value::list(
                            vec![
                                Value::test_string("ls"),
                                Value::test_string("mv"),
                                Value::test_string("cd"),
                            ],
                            Span::test_data(),
                        ),
                        Value::list(
                            vec![
                                Value::test_string("list"),
                                Value::test_string("move"),
                                Value::test_string("change directory"),
                            ],
                            Span::test_data(),
                        ),
                    ],
                    Span::test_data(),
                )),
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
        values(engine_state, span, input)
    }
}

// The semantics of `values` are as follows:
// For each column, get the values for that column, in row order.
// Holes are not preserved, i.e. position in the resulting list
// does not necessarily equal row number.
pub fn get_values<'a>(
    input: impl IntoIterator<Item = &'a Value>,
    head: Span,
    input_span: Span,
) -> Result<Vec<Value>, ShellError> {
    let mut output: IndexMap<String, Vec<Value>> = IndexMap::new();

    for item in input {
        match item {
            Value::Record { val, .. } => {
                for (k, v) in &**val {
                    if let Some(vec) = output.get_mut(k) {
                        vec.push(v.clone());
                    } else {
                        output.insert(k.clone(), vec![v.clone()]);
                    }
                }
            }
            Value::Error { error, .. } => return Err(*error.clone()),
            _ => {
                return Err(ShellError::OnlySupportsThisInputType {
                    exp_input_type: "record or table".into(),
                    wrong_type: item.get_type().to_string(),
                    dst_span: head,
                    src_span: input_span,
                });
            }
        }
    }

    Ok(output.into_values().map(|v| Value::list(v, head)).collect())
}

fn values(
    engine_state: &EngineState,
    head: Span,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let signals = engine_state.signals().clone();
    let metadata = input.metadata();
    match input {
        PipelineData::Empty => Ok(PipelineData::empty()),
        PipelineData::Value(v, ..) => {
            let span = v.span();
            match v {
                Value::List { vals, .. } => match get_values(&vals, head, span) {
                    Ok(cols) => Ok(cols
                        .into_iter()
                        .into_pipeline_data_with_metadata(head, signals, metadata)),
                    Err(err) => Err(err),
                },
                Value::Custom { val, .. } => {
                    let input_as_base_value = val.to_base_value(span)?;
                    match get_values(&[input_as_base_value], head, span) {
                        Ok(cols) => Ok(cols
                            .into_iter()
                            .into_pipeline_data_with_metadata(head, signals, metadata)),
                        Err(err) => Err(err),
                    }
                }
                Value::Record { val, .. } => Ok(val
                    .values()
                    .cloned()
                    .collect::<Vec<_>>()
                    .into_pipeline_data_with_metadata(head, signals, metadata)),
                // Propagate errors
                Value::Error { error, .. } => Err(*error),
                other => Err(ShellError::OnlySupportsThisInputType {
                    exp_input_type: "record or table".into(),
                    wrong_type: other.get_type().to_string(),
                    dst_span: head,
                    src_span: other.span(),
                }),
            }
        }
        PipelineData::ListStream(stream, ..) => {
            let vals: Vec<_> = stream.into_iter().collect();
            match get_values(&vals, head, head) {
                Ok(cols) => Ok(cols
                    .into_iter()
                    .into_pipeline_data_with_metadata(head, signals, metadata)),
                Err(err) => Err(err),
            }
        }
        PipelineData::ByteStream(stream, ..) => Err(ShellError::OnlySupportsThisInputType {
            exp_input_type: "record or table".into(),
            wrong_type: stream.type_().describe().into(),
            dst_span: head,
            src_span: stream.span(),
        }),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Values {})
    }
}
