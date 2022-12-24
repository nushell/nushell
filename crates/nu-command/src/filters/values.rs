use indexmap::IndexMap;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Span,
    Type, Value,
};

#[derive(Clone)]
pub struct Values;

impl Command for Values {
    fn name(&self) -> &str {
        "values"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (Type::Record(vec![]), Type::List(Box::new(Type::Any))),
                (Type::Table(vec![]), Type::List(Box::new(Type::Any))),
            ])
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Given a record or table, produce a list of its columns' values."
    }

    fn extra_usage(&self) -> &str {
        "This is a counterpart to `columns`, which produces a list of columns' names."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "{ mode:normal userid:31415 } | values",
                description: "Get the values from the record (produce a list)",
                result: Some(Value::List {
                    vals: vec![Value::test_string("normal"), Value::test_int(31415)],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "{ f:250 g:191 c:128 d:1024 e:2000 a:16 b:32 } | values",
                description: "Values are ordered by the column order of the record",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_int(250),
                        Value::test_int(191),
                        Value::test_int(128),
                        Value::test_int(1024),
                        Value::test_int(2000),
                        Value::test_int(16),
                        Value::test_int(32),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[[name meaning]; [ls list] [mv move] [cd 'change directory']] | values",
                description: "Get the values from the table (produce a list of lists)",
                result: Some(Value::List {
                    vals: vec![
                        Value::List {
                            vals: vec![
                                Value::test_string("ls"),
                                Value::test_string("mv"),
                                Value::test_string("cd"),
                            ],
                            span: Span::test_data(),
                        },
                        Value::List {
                            vals: vec![
                                Value::test_string("list"),
                                Value::test_string("move"),
                                Value::test_string("change directory"),
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
            Value::Record { cols, vals, .. } => {
                for (k, v) in cols.iter().zip(vals.iter()) {
                    if let Some(vec) = output.get_mut(k) {
                        vec.push(v.clone());
                    } else {
                        output.insert(k.clone(), vec![v.clone()]);
                    }
                }
            }
            Value::Error { error } => return Err(error.clone()),
            _ => {
                return Err(ShellError::OnlySupportsThisInputType(
                    "record or table".into(),
                    item.get_type().to_string(),
                    head,
                    input_span,
                ))
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
    let ctrlc = engine_state.ctrlc.clone();
    let metadata = input.metadata();
    match input {
        PipelineData::Empty => Ok(PipelineData::Empty),
        PipelineData::Value(Value::List { vals, span }, ..) => {
            match get_values(&vals, head, span) {
                Ok(cols) => Ok(cols
                    .into_iter()
                    .into_pipeline_data(ctrlc)
                    .set_metadata(metadata)),
                Err(err) => Err(err),
            }
        }
        PipelineData::Value(Value::CustomValue { val, span }, ..) => {
            let input_as_base_value = val.to_base_value(span)?;
            match get_values(&[input_as_base_value], head, span) {
                Ok(cols) => Ok(cols
                    .into_iter()
                    .into_pipeline_data(ctrlc)
                    .set_metadata(metadata)),
                Err(err) => Err(err),
            }
        }
        PipelineData::ListStream(stream, ..) => {
            let vals: Vec<_> = stream.into_iter().collect();
            match get_values(&vals, head, head) {
                Ok(cols) => Ok(cols
                    .into_iter()
                    .into_pipeline_data(ctrlc)
                    .set_metadata(metadata)),
                Err(err) => Err(err),
            }
        }
        PipelineData::Value(Value::Record { vals, .. }, ..) => {
            Ok(vals.into_pipeline_data(ctrlc).set_metadata(metadata))
        }
        // Propagate errors
        PipelineData::Value(Value::Error { error }, ..) => Err(error),
        PipelineData::Value(other, ..) => {
            Err(ShellError::OnlySupportsThisInputType(
                "record or table".into(),
                other.get_type().to_string(),
                head,
                // This line requires the Value::Error match above.
                other.expect_span(),
            ))
        }
        PipelineData::ExternalStream { .. } => {
            Err(ShellError::OnlySupportsThisInputType(
                "record or table".into(),
                "raw data".into(),
                head,
                // This line requires the PipelineData::Empty and PipelineData::ListStream matches above.
                input
                    .span()
                    .expect("PipelineData::ExternalStream had no span"),
            ))
        }
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
