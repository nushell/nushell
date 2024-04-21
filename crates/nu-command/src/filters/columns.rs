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

    fn usage(&self) -> &str {
        "Given a record or table, produce a list of its columns' names."
    }

    fn extra_usage(&self) -> &str {
        "This is a counterpart to `values`, which produces a list of columns' values."
    }

    fn examples(&self) -> Vec<Example> {
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
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        getcol(engine_state, span, input)
    }
}

fn getcol(
    engine_state: &EngineState,
    head: Span,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let ctrlc = engine_state.ctrlc.clone();
    let metadata = input.metadata();
    match input {
        PipelineData::Empty => Ok(PipelineData::Empty),
        PipelineData::Value(v, ..) => {
            let span = v.span();
            match v {
                Value::List {
                    vals: input_vals, ..
                } => {
                    let input_cols = get_columns(&input_vals);
                    Ok(input_cols
                        .into_iter()
                        .map(move |x| Value::string(x, span))
                        .into_pipeline_data(ctrlc)
                        .set_metadata(metadata))
                }
                Value::Custom { val, .. } => {
                    // TODO: should we get CustomValue to expose columns in a more efficient way?
                    // Would be nice to be able to get columns without generating the whole value
                    let input_as_base_value = val.to_base_value(span)?;
                    let input_cols = get_columns(&[input_as_base_value]);
                    Ok(input_cols
                        .into_iter()
                        .map(move |x| Value::string(x, span))
                        .into_pipeline_data(ctrlc)
                        .set_metadata(metadata))
                }
                Value::LazyRecord { val, .. } => {
                    Ok({
                        // Unfortunate casualty to LazyRecord's column_names not generating 'static strs
                        let cols: Vec<_> =
                            val.column_names().iter().map(|s| s.to_string()).collect();

                        cols.into_iter()
                            .map(move |x| Value::string(x, head))
                            .into_pipeline_data(ctrlc)
                            .set_metadata(metadata)
                    })
                }
                Value::Record { val, .. } => Ok(val
                    .into_owned()
                    .into_iter()
                    .map(move |(x, _)| Value::string(x, head))
                    .into_pipeline_data(ctrlc)
                    .set_metadata(metadata)),
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
            let v: Vec<_> = stream.into_iter().collect();
            let input_cols = get_columns(&v);

            Ok(input_cols
                .into_iter()
                .map(move |x| Value::string(x, head))
                .into_pipeline_data_with_metadata(metadata, ctrlc))
        }
        PipelineData::ExternalStream { .. } => Err(ShellError::OnlySupportsThisInputType {
            exp_input_type: "record or table".into(),
            wrong_type: "raw data".into(),
            dst_span: head,
            src_span: input
                .span()
                .expect("PipelineData::ExternalStream had no span"),
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
