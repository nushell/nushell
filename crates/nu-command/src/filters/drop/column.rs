use nu_engine::command_prelude::*;

use std::collections::HashSet;

#[derive(Clone)]
pub struct DropColumn;

impl Command for DropColumn {
    fn name(&self) -> &str {
        "drop column"
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
                "Starting from the end, the number of columns to remove.",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Remove N columns at the right-hand end of the input table. To remove columns by name, use `reject`."
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
        // the number of columns to drop
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

        drop_cols(engine_state, input, call.head, columns)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Remove the last column of a table",
                example: "[[lib, extension]; [nu-lib, rs] [nu-core, rb]] | drop column",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! { "lib" => Value::test_string("nu-lib") }),
                    Value::test_record(record! { "lib" => Value::test_string("nu-core") }),
                ])),
            },
            Example {
                description: "Remove the last column of a record",
                example: "{lib: nu-lib, extension: rs} | drop column",
                result: Some(Value::test_record(
                    record! { "lib" => Value::test_string("nu-lib") },
                )),
            },
        ]
    }
}

fn drop_cols(
    engine_state: &EngineState,
    input: PipelineData,
    head: Span,
    columns: usize,
) -> Result<PipelineData, ShellError> {
    // For simplicity and performance, we use the first row's columns
    // as the columns for the whole table, and assume that later rows/records
    // have these same columns. However, this can give weird results like:
    // `[{a: 1}, {b: 2}] | drop column`
    // This will drop the column "a" instead of "b" even though column "b"
    // is displayed farther to the right.
    let metadata = input.metadata();
    match input {
        PipelineData::ListStream(stream, ..) => {
            let mut stream = stream.into_iter();
            if let Some(mut first) = stream.next() {
                let drop_cols = drop_cols_set(&mut first, head, columns)?;

                Ok(std::iter::once(first)
                    .chain(stream.map(move |mut v| {
                        match drop_record_cols(&mut v, head, &drop_cols) {
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
                        let drop_cols = drop_cols_set(first, head, columns)?;
                        for val in rest {
                            drop_record_cols(val, head, &drop_cols)?
                        }
                    }
                    Ok(Value::list(vals, span).into_pipeline_data_with_metadata(metadata))
                }
                Value::Record {
                    val: ref mut record,
                    ..
                } => {
                    let len = record.len().saturating_sub(columns);
                    record.to_mut().truncate(len);
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

fn drop_cols_set(val: &mut Value, head: Span, drop: usize) -> Result<HashSet<String>, ShellError> {
    if let Value::Record { val: record, .. } = val {
        let len = record.len().saturating_sub(drop);
        Ok(record.to_mut().drain(len..).map(|(col, _)| col).collect())
    } else {
        Err(unsupported_value_error(val, head))
    }
}

fn drop_record_cols(
    val: &mut Value,
    head: Span,
    drop_cols: &HashSet<String>,
) -> Result<(), ShellError> {
    if let Value::Record { val, .. } = val {
        val.to_mut().retain(|col, _| !drop_cols.contains(col));
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
        crate::test_examples(DropColumn)
    }
}
