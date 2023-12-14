use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    record, Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData,
    Record, ShellError, Signature, Span, Spanned, SyntaxShape, Type, Value,
};

#[derive(Clone, Debug)]
enum BeforeOrAfter {
    Before(String),
    After(String),
}

#[derive(Clone)]
pub struct Move;

impl Command for Move {
    fn name(&self) -> &str {
        "move"
    }

    fn usage(&self) -> &str {
        "Move columns before or after other columns."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("move")
            .input_output_types(vec![
                (Type::Record(vec![]), Type::Record(vec![])),
                (Type::Table(vec![]), Type::Table(vec![])),
            ])
            .rest("columns", SyntaxShape::String, "the columns to move")
            .named(
                "after",
                SyntaxShape::String,
                "the column that will precede the columns moved",
                None,
            )
            .named(
                "before",
                SyntaxShape::String,
                "the column that will be the next after the columns moved",
                None,
            )
            .category(Category::Filters)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[[name value index]; [foo a 1] [bar b 2] [baz c 3]] | move index --before name",
                description: "Move a column before the first column",
                result:
                    Some(Value::test_list(
                        vec![
                            Value::test_record(record! {
                                "index" => Value::test_int(1),
                                "name" =>  Value::test_string("foo"),
                                "value" => Value::test_string("a"),
                            }),
                            Value::test_record(record! {
                                "index" => Value::test_int(2),
                                "name" =>  Value::test_string("bar"),
                                "value" => Value::test_string("b"),
                            }),
                            Value::test_record(record! {
                                "index" => Value::test_int(3),
                                "name" =>  Value::test_string("baz"),
                                "value" => Value::test_string("c"),
                            }),
                        ],
                    ))
            },
            Example {
                example: "[[name value index]; [foo a 1] [bar b 2] [baz c 3]] | move value name --after index",
                description: "Move multiple columns after the last column and reorder them",
                result:
                    Some(Value::test_list(
                        vec![
                            Value::test_record(record! {
                                "index" => Value::test_int(1),
                                "value" => Value::test_string("a"),
                                "name" =>  Value::test_string("foo"),
                            }),
                            Value::test_record(record! {
                                "index" => Value::test_int(2),
                                "value" => Value::test_string("b"),
                                "name" =>  Value::test_string("bar"),
                            }),
                            Value::test_record(record! {
                                "index" => Value::test_int(3),
                                "value" => Value::test_string("c"),
                                "name" =>  Value::test_string("baz"),
                            }),
                        ],
                    ))
            },
            Example {
                example: "{ name: foo, value: a, index: 1 } | move name --before index",
                description: "Move columns of a record",
                result: Some(Value::test_record(record! {
                    "value" => Value::test_string("a"),
                    "name" => Value::test_string("foo"),
                    "index" => Value::test_int(1),
                }))
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let columns: Vec<Value> = call.rest(engine_state, stack, 0)?;
        let after: Option<Value> = call.get_flag(engine_state, stack, "after")?;
        let before: Option<Value> = call.get_flag(engine_state, stack, "before")?;

        let before_or_after = match (after, before) {
            (Some(v), None) => Spanned {
                item: BeforeOrAfter::After(v.as_string()?),
                span: v.span(),
            },
            (None, Some(v)) => Spanned {
                item: BeforeOrAfter::Before(v.as_string()?),
                span: v.span(),
            },
            (Some(_), Some(_)) => {
                return Err(ShellError::GenericError {
                    error: "Cannot move columns".into(),
                    msg: "Use either --after, or --before, not both".into(),
                    span: Some(call.head),
                    help: None,
                    inner: vec![],
                })
            }
            (None, None) => {
                return Err(ShellError::GenericError {
                    error: "Cannot move columns".into(),
                    msg: "Missing --after or --before flag".into(),
                    span: Some(call.head),
                    help: None,
                    inner: vec![],
                })
            }
        };

        let metadata = input.metadata();
        let ctrlc = engine_state.ctrlc.clone();
        let call = call.clone();

        match input {
            PipelineData::Value(Value::List { .. }, ..) | PipelineData::ListStream { .. } => {
                let res = input.into_iter().map(move |x| match x.as_record() {
                    Ok(record) => {
                        match move_record_columns(record, &columns, &before_or_after, call.head) {
                            Ok(val) => val,
                            Err(error) => Value::error(error, call.head),
                        }
                    }
                    Err(error) => Value::error(error, call.head),
                });

                if let Some(md) = metadata {
                    Ok(res.into_pipeline_data_with_metadata(md, ctrlc))
                } else {
                    Ok(res.into_pipeline_data(ctrlc))
                }
            }
            PipelineData::Value(Value::Record { val, .. }, ..) => {
                Ok(
                    move_record_columns(&val, &columns, &before_or_after, call.head)?
                        .into_pipeline_data(),
                )
            }
            _ => Err(ShellError::PipelineMismatch {
                exp_input_type: "record or table".to_string(),
                dst_span: call.head,
                src_span: Span::new(call.head.start, call.head.start),
            }),
        }
    }
}

// Move columns within a record
fn move_record_columns(
    record: &Record,
    columns: &[Value],
    before_or_after: &Spanned<BeforeOrAfter>,
    span: Span,
) -> Result<Value, ShellError> {
    let mut column_idx: Vec<usize> = Vec::with_capacity(columns.len());

    // Check if before/after column exist
    match &before_or_after.item {
        BeforeOrAfter::After(after) => {
            if !record.contains(after) {
                return Err(ShellError::GenericError {
                    error: "Cannot move columns".into(),
                    msg: "column does not exist".into(),
                    span: Some(before_or_after.span),
                    help: None,
                    inner: vec![],
                });
            }
        }
        BeforeOrAfter::Before(before) => {
            if !record.contains(before) {
                return Err(ShellError::GenericError {
                    error: "Cannot move columns".into(),
                    msg: "column does not exist".into(),
                    span: Some(before_or_after.span),
                    help: None,
                    inner: vec![],
                });
            }
        }
    }

    // Find indices of columns to be moved
    for column in columns.iter() {
        let column_str = column.as_string()?;

        if let Some(idx) = record.index_of(&column_str) {
            column_idx.push(idx);
        } else {
            return Err(ShellError::GenericError {
                error: "Cannot move columns".into(),
                msg: "column does not exist".into(),
                span: Some(column.span()),
                help: None,
                inner: vec![],
            });
        }
    }

    let mut out = Record::with_capacity(record.len());

    for (i, (inp_col, inp_val)) in record.iter().enumerate() {
        match &before_or_after.item {
            BeforeOrAfter::After(after) if after == inp_col => {
                out.push(inp_col.clone(), inp_val.clone());

                for idx in column_idx.iter() {
                    if let Some((col, val)) = record.get_index(*idx) {
                        out.push(col.clone(), val.clone());
                    } else {
                        return Err(ShellError::NushellFailedSpanned {
                            msg: "Error indexing input columns".to_string(),
                            label: "originates from here".to_string(),
                            span,
                        });
                    }
                }
            }
            BeforeOrAfter::Before(before) if before == inp_col => {
                for idx in column_idx.iter() {
                    if let Some((col, val)) = record.get_index(*idx) {
                        out.push(col.clone(), val.clone());
                    } else {
                        return Err(ShellError::NushellFailedSpanned {
                            msg: "Error indexing input columns".to_string(),
                            label: "originates from here".to_string(),
                            span,
                        });
                    }
                }

                out.push(inp_col.clone(), inp_val.clone());
            }
            _ => {
                if !column_idx.contains(&i) {
                    out.push(inp_col.clone(), inp_val.clone());
                }
            }
        }
    }

    Ok(Value::record(out, span))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Move {})
    }
}
