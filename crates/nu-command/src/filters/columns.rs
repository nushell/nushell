use nu_engine::{column::get_columns, command_prelude::*};

#[derive(Clone)]
pub struct Columns;

impl Command for Columns {
    fn name(&self) -> &str {
        "columns"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (Type::table(), Type::List(Box::new(Type::String))),
                (Type::record(), Type::List(Box::new(Type::String))),
            ])
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Given a record or table, produce a list of its columns' names."
    }

    fn extra_description(&self) -> &str {
        "This is a counterpart to `values`, which produces a list of columns' values."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "{ acronym:PWD, meaning:'Print Working Directory' } | columns",
                description: "Get the columns from the record",
                result: Some(Value::list(
                    vec![Value::test_string("acronym"), Value::test_string("meaning")],
                    Span::test_data(),
                )),
            },
            Example {
                example: "[[name,age,grade]; [bill,20,a]] | columns",
                description: "Get the columns from the table",
                result: Some(Value::list(
                    vec![
                        Value::test_string("name"),
                        Value::test_string("age"),
                        Value::test_string("grade"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                example: "[[name,age,grade]; [bill,20,a]] | columns | first",
                description: "Get the first column from the table",
                result: None,
            },
            Example {
                example: "[[name,age,grade]; [bill,20,a]] | columns | select 1",
                description: "Get the second column from the table",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        getcol(call.head, input)
    }
}

fn getcol(head: Span, input: PipelineData) -> Result<PipelineData, ShellError> {
    let metadata = input.metadata();
    match input {
        PipelineData::Empty => Ok(PipelineData::empty()),
        PipelineData::Value(v, ..) => {
            let span = v.span();
            let cols = match v {
                Value::List {
                    vals: input_vals, ..
                } => get_columns(&input_vals)
                    .into_iter()
                    .map(move |x| Value::string(x, span))
                    .collect(),
                Value::Custom { val, .. } => {
                    // TODO: should we get CustomValue to expose columns in a more efficient way?
                    // Would be nice to be able to get columns without generating the whole value
                    let input_as_base_value = val.to_base_value(span)?;
                    get_columns(&[input_as_base_value])
                        .into_iter()
                        .map(move |x| Value::string(x, span))
                        .collect()
                }
                Value::Record { val, .. } => val
                    .into_owned()
                    .into_iter()
                    .map(move |(x, _)| Value::string(x, head))
                    .collect(),
                // Propagate errors
                Value::Error { error, .. } => return Err(*error),
                other => {
                    return Err(ShellError::OnlySupportsThisInputType {
                        exp_input_type: "record or table".into(),
                        wrong_type: other.get_type().to_string(),
                        dst_span: head,
                        src_span: other.span(),
                    });
                }
            };

            Ok(Value::list(cols, head)
                .into_pipeline_data()
                .set_metadata(metadata))
        }
        PipelineData::ListStream(stream, ..) => {
            let values = stream.into_iter().collect::<Vec<_>>();
            let cols = get_columns(&values)
                .into_iter()
                .map(|s| Value::string(s, head))
                .collect();

            Ok(Value::list(cols, head)
                .into_pipeline_data()
                .set_metadata(metadata))
        }
        PipelineData::ByteStream(stream, ..) => Err(ShellError::OnlySupportsThisInputType {
            exp_input_type: "record or table".into(),
            wrong_type: "byte stream".into(),
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

        test_examples(Columns {})
    }
}
