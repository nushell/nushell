use nu_engine::command_prelude::*;

use std::collections::HashSet;

#[derive(Clone)]
pub struct SkipColumn;

impl Command for SkipColumn {
    fn name(&self) -> &str {
        "skip column"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .optional(
                "columns",
                SyntaxShape::Int,
                "Starting from the beginning, the number of columns to remove.",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Remove N columns at the left-hand end of the input table. To remove columns by name, use `reject`."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["delete", "remove"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // the number of columns to skip
        let columns: Option<Spanned<i64>> = call.opt(engine_state, stack, 0)?;

        let columns = if let Some(columns) = columns {
            if columns.item < 0 {
                return Err(ShellError::NeedsPositiveValue { span: columns.span });
            } else {
                columns.item as usize
            }
        } else {
            1
        };

        skip_cols(engine_state, input, call.head, columns)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Remove the first column of a table",
                example: "[[lib, extension]; [nu-lib, rs] [nu-core, rb]] | skip column",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! { "extension" => Value::test_string("rs") }),
                    Value::test_record(record! { "extension" => Value::test_string("rb") }),
                ])),
            },
            Example {
                description: "Remove the first column of a record",
                example: "{lib: nu-lib, extension: rs} | skip column",
                result: Some(Value::test_record(
                    record! { "extension" => Value::test_string("rs") },
                )),
            },
        ]
    }
}

fn skip_cols(
    engine_state: &EngineState,
    input: PipelineData,
    head: Span,
    columns: usize,
) -> Result<PipelineData, ShellError> {
    let metadata = input.metadata();
    match input {
        PipelineData::ListStream(stream, ..) => {
            let mut stream = stream.into_iter();
            if let Some(mut first) = stream.next() {
                let skip_cols = skip_cols_set(&mut first, head, columns)?;

                Ok(std::iter::once(first)
                    .chain(stream.map(move |mut v| {
                        match skip_record_cols(&mut v, head, &skip_cols) {
                            Ok(()) => v,
                            Err(e) => Value::error(e, head),
                        }
                    }))
                    .into_pipeline_data_with_metadata(
                        head,
                        engine_state.signals().clone(),
                        metadata,
                    ))
            } else {
                Ok(PipelineData::empty())
            }
        }
        PipelineData::Value(mut v, ..) => {
            let span = v.span();
            match v {
                Value::List { mut vals, .. } => {
                    if let Some((first, rest)) = vals.split_first_mut() {
                        let skip_cols = skip_cols_set(first, head, columns)?;
                        for val in rest {
                            skip_record_cols(val, head, &skip_cols)?
                        }
                    }
                    Ok(Value::list(vals, span).into_pipeline_data_with_metadata(metadata))
                }
                Value::Record {
                    val: ref mut record,
                    ..
                } => {
                    let len = record.len().saturating_sub(columns);
                    record.to_mut().truncate_front(len);
                    Ok(v.into_pipeline_data_with_metadata(metadata))
                }
                // Propagate errors
                Value::Error { error, .. } => Err(*error),
                val => Err(unsupported_value_error(&val, head)),
            }
        }
        PipelineData::Empty => Ok(PipelineData::empty()),
        PipelineData::ByteStream(stream, ..) => Err(ShellError::OnlySupportsThisInputType {
            exp_input_type: "table or record".into(),
            wrong_type: stream.type_().describe().into(),
            dst_span: head,
            src_span: stream.span(),
        }),
    }
}

fn skip_cols_set(val: &mut Value, head: Span, skip: usize) -> Result<HashSet<String>, ShellError> {
    if let Value::Record { val: record, .. } = val {
        Ok(record.to_mut().drain(..skip).map(|(col, _)| col).collect())
    } else {
        Err(unsupported_value_error(val, head))
    }
}

fn skip_record_cols(
    val: &mut Value,
    head: Span,
    skip_cols: &HashSet<String>,
) -> Result<(), ShellError> {
    if let Value::Record { val, .. } = val {
        val.to_mut().retain(|col, _| !skip_cols.contains(col));
        Ok(())
    } else {
        Err(unsupported_value_error(val, head))
    }
}

fn unsupported_value_error(val: &Value, head: Span) -> ShellError {
    ShellError::OnlySupportsThisInputType {
        exp_input_type: "table or record".into(),
        wrong_type: val.get_type().to_string(),
        dst_span: head,
        src_span: val.span(),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        crate::test_examples(SkipColumn)
    }
}
