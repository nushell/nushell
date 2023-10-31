use std::collections::HashSet;
use std::iter;

use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    record, Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData,
    ShellError, Signature, Span, Spanned, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct DropColumn;

impl Command for DropColumn {
    fn name(&self) -> &str {
        "drop column"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (Type::Table(vec![]), Type::Table(vec![])),
                (Type::Record(vec![]), Type::Record(vec![])),
            ])
            .optional(
                "columns",
                SyntaxShape::Int,
                "starting from the end, the number of columns to remove",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Remove N columns at the right-hand end of the input table. To remove columns by name, use `reject`."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["delete"]
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
                return Err(ShellError::NeedsPositiveValue(columns.span));
            } else {
                columns.item as usize
            }
        } else {
            1
        };

        drop_cols(engine_state, input, call.head, columns)
    }

    fn examples(&self) -> Vec<Example> {
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
    match input {
        PipelineData::ListStream(mut stream, ..) => {
            if let Some(mut first) = stream.next() {
                let drop_cols = drop_cols_set(&mut first, head, columns)?;

                Ok(iter::once(first)
                    .chain(stream.map(move |mut v| {
                        match drop_record_cols(&mut v, head, &drop_cols) {
                            Ok(()) => v,
                            Err(e) => Value::error(e, head),
                        }
                    }))
                    .into_pipeline_data(engine_state.ctrlc.clone()))
            } else {
                Ok(PipelineData::Empty)
            }
        }
        PipelineData::Value(v, ..) => {
            let span = v.span();
            match v {
                Value::List { mut vals, .. } => {
                    if let Some((first, rest)) = vals.split_first_mut() {
                        let drop_cols = drop_cols_set(first, head, columns)?;
                        for val in rest {
                            drop_record_cols(val, head, &drop_cols)?
                        }
                    }
                    Ok(Value::list(vals, span).into_pipeline_data())
                }
                Value::Record {
                    val: mut record, ..
                } => {
                    let len = record.len().saturating_sub(columns);
                    record.cols.truncate(len);
                    record.vals.truncate(len);
                    Ok(Value::record(record, span).into_pipeline_data())
                }
                // Propagate errors
                Value::Error { error, .. } => Err(*error),
                val => Err(unsupported_value_error(&val, head)),
            }
        }
        PipelineData::Empty => Ok(PipelineData::Empty),
        PipelineData::ExternalStream { span, .. } => Err(ShellError::OnlySupportsThisInputType {
            exp_input_type: "table or record".into(),
            wrong_type: "raw data".into(),
            dst_span: head,
            src_span: span,
        }),
    }
}

fn drop_cols_set(val: &mut Value, head: Span, drop: usize) -> Result<HashSet<String>, ShellError> {
    if let Value::Record { val: record, .. } = val {
        let len = record.len().saturating_sub(drop);
        record.vals.truncate(len);
        Ok(record.cols.drain(len..).collect())
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
        // TOOO: Needs `Record::retain` to be performant,
        // since this is currently O(n^2)
        // where n is the number of columns being dropped.
        // (Assuming dropped columns are at the end of the record.)
        val.retain(|col, _| !drop_cols.contains(col));
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
