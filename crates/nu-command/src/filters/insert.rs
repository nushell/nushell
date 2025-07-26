use std::borrow::Cow;

use nu_engine::{ClosureEval, ClosureEvalOnce, command_prelude::*};
use nu_protocol::ast::PathMember;

#[derive(Clone)]
pub struct Insert;

impl Command for Insert {
    fn name(&self) -> &str {
        "insert"
    }

    fn signature(&self) -> Signature {
        Signature::build("insert")
            .input_output_types(vec![
                (Type::record(), Type::record()),
                (Type::table(), Type::table()),
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
            ])
            .required(
                "field",
                SyntaxShape::CellPath,
                "The name of the column to insert.",
            )
            .required(
                "new value",
                SyntaxShape::Any,
                "The new value to give the cell(s).",
            )
            .allow_variants_without_examples(true)
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Insert a new column, using an expression or closure to create each row's values."
    }

    fn extra_description(&self) -> &str {
        "When inserting a column, the closure will be run for each row, and the current row will be passed as the first argument.
When inserting into a specific index, the closure will instead get the current value at the index or null if inserting at the end of a list/table."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["add"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        insert(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Insert a new entry into a single record",
                example: "{'name': 'nu', 'stars': 5} | insert alias 'Nushell'",
                result: Some(Value::test_record(record! {
                    "name" => Value::test_string("nu"),
                    "stars" => Value::test_int(5),
                    "alias" => Value::test_string("Nushell"),
                })),
            },
            Example {
                description: "Insert a new column into a table, populating all rows",
                example: "[[project, lang]; ['Nushell', 'Rust']] | insert type 'shell'",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "project" => Value::test_string("Nushell"),
                    "lang" => Value::test_string("Rust"),
                    "type" => Value::test_string("shell"),
                })])),
            },
            Example {
                description: "Insert a new column with values computed based off the other columns",
                example: "[[foo]; [7] [8] [9]] | insert bar {|row| $row.foo * 2 }",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "foo" => Value::test_int(7),
                        "bar" => Value::test_int(14),
                    }),
                    Value::test_record(record! {
                        "foo" => Value::test_int(8),
                        "bar" => Value::test_int(16),
                    }),
                    Value::test_record(record! {
                        "foo" => Value::test_int(9),
                        "bar" => Value::test_int(18),
                    }),
                ])),
            },
            Example {
                description: "Insert a new value into a list at an index",
                example: "[1 2 4] | insert 2 3",
                result: Some(Value::test_list(vec![
                    Value::test_int(1),
                    Value::test_int(2),
                    Value::test_int(3),
                    Value::test_int(4),
                ])),
            },
            Example {
                description: "Insert a new value at the end of a list",
                example: "[1 2 3] | insert 3 4",
                result: Some(Value::test_list(vec![
                    Value::test_int(1),
                    Value::test_int(2),
                    Value::test_int(3),
                    Value::test_int(4),
                ])),
            },
            Example {
                description: "Insert into a nested path, creating new values as needed",
                example: "[{} {a: [{}]}] | insert a.0.b \"value\"",
                result: Some(Value::test_list(vec![
                    Value::test_record(record!(
                        "a" => Value::test_list(vec![Value::test_record(record!(
                            "b" => Value::test_string("value"),
                        ))]),
                    )),
                    Value::test_record(record!(
                        "a" => Value::test_list(vec![Value::test_record(record!(
                            "b" => Value::test_string("value"),
                        ))]),
                    )),
                ])),
            },
        ]
    }
}

fn insert(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let cell_path: CellPath = call.req(engine_state, stack, 0)?;
    let replacement: Value = call.req(engine_state, stack, 1)?;

    match input {
        // Propagate errors in the pipeline
        PipelineData::Value(Value::Error { error, .. }, ..) => Err(*error),
        PipelineData::Value(mut value, metadata) => {
            if let Value::Closure { val, .. } = replacement {
                match (cell_path.members.first(), &mut value) {
                    (Some(PathMember::String { .. }), Value::List { vals, .. }) => {
                        let mut closure = ClosureEval::new(engine_state, stack, *val);
                        for val in vals {
                            insert_value_by_closure(
                                val,
                                &mut closure,
                                head,
                                &cell_path.members,
                                false,
                            )?;
                        }
                    }
                    (first, _) => {
                        insert_single_value_by_closure(
                            &mut value,
                            ClosureEvalOnce::new(engine_state, stack, *val),
                            head,
                            &cell_path.members,
                            matches!(first, Some(PathMember::Int { .. })),
                        )?;
                    }
                }
            } else {
                value.insert_data_at_cell_path(&cell_path.members, replacement, head)?;
            }
            Ok(value.into_pipeline_data_with_metadata(metadata))
        }
        PipelineData::ListStream(stream, metadata) => {
            if let Some((
                &PathMember::Int {
                    val,
                    span: path_span,
                    ..
                },
                path,
            )) = cell_path.members.split_first()
            {
                let mut stream = stream.into_iter();
                let mut pre_elems = vec![];

                for idx in 0..val {
                    if let Some(v) = stream.next() {
                        pre_elems.push(v);
                    } else {
                        return Err(ShellError::InsertAfterNextFreeIndex {
                            available_idx: idx,
                            span: path_span,
                        });
                    }
                }

                if path.is_empty() {
                    if let Value::Closure { val, .. } = replacement {
                        let value = stream.next();
                        let end_of_stream = value.is_none();
                        let value = value.unwrap_or(Value::nothing(head));
                        let new_value = ClosureEvalOnce::new(engine_state, stack, *val)
                            .run_with_value(value.clone())?
                            .into_value(head)?;

                        pre_elems.push(new_value);
                        if !end_of_stream {
                            pre_elems.push(value);
                        }
                    } else {
                        pre_elems.push(replacement);
                    }
                } else if let Some(mut value) = stream.next() {
                    if let Value::Closure { val, .. } = replacement {
                        insert_single_value_by_closure(
                            &mut value,
                            ClosureEvalOnce::new(engine_state, stack, *val),
                            head,
                            path,
                            true,
                        )?;
                    } else {
                        value.insert_data_at_cell_path(path, replacement, head)?;
                    }
                    pre_elems.push(value)
                } else {
                    return Err(ShellError::AccessBeyondEnd {
                        max_idx: pre_elems.len() - 1,
                        span: path_span,
                    });
                }

                Ok(pre_elems
                    .into_iter()
                    .chain(stream)
                    .into_pipeline_data_with_metadata(
                        head,
                        engine_state.signals().clone(),
                        metadata,
                    ))
            } else if let Value::Closure { val, .. } = replacement {
                let mut closure = ClosureEval::new(engine_state, stack, *val);
                let stream = stream.map(move |mut value| {
                    let err = insert_value_by_closure(
                        &mut value,
                        &mut closure,
                        head,
                        &cell_path.members,
                        false,
                    );

                    if let Err(e) = err {
                        Value::error(e, head)
                    } else {
                        value
                    }
                });
                Ok(PipelineData::list_stream(stream, metadata))
            } else {
                let stream = stream.map(move |mut value| {
                    if let Err(e) = value.insert_data_at_cell_path(
                        &cell_path.members,
                        replacement.clone(),
                        head,
                    ) {
                        Value::error(e, head)
                    } else {
                        value
                    }
                });

                Ok(PipelineData::list_stream(stream, metadata))
            }
        }
        PipelineData::Empty => Err(ShellError::IncompatiblePathAccess {
            type_name: "empty pipeline".to_string(),
            span: head,
        }),
        PipelineData::ByteStream(stream, ..) => Err(ShellError::IncompatiblePathAccess {
            type_name: stream.type_().describe().into(),
            span: head,
        }),
    }
}

fn insert_value_by_closure(
    value: &mut Value,
    closure: &mut ClosureEval,
    span: Span,
    cell_path: &[PathMember],
    first_path_member_int: bool,
) -> Result<(), ShellError> {
    let value_at_path = if first_path_member_int {
        value
            .follow_cell_path(cell_path)
            .map(Cow::into_owned)
            .unwrap_or(Value::nothing(span))
    } else {
        value.clone()
    };

    let new_value = closure.run_with_value(value_at_path)?.into_value(span)?;
    value.insert_data_at_cell_path(cell_path, new_value, span)
}

fn insert_single_value_by_closure(
    value: &mut Value,
    closure: ClosureEvalOnce,
    span: Span,
    cell_path: &[PathMember],
    first_path_member_int: bool,
) -> Result<(), ShellError> {
    let value_at_path = if first_path_member_int {
        value
            .follow_cell_path(cell_path)
            .map(Cow::into_owned)
            .unwrap_or(Value::nothing(span))
    } else {
        value.clone()
    };

    let new_value = closure.run_with_value(value_at_path)?.into_value(span)?;
    value.insert_data_at_cell_path(cell_path, new_value, span)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Insert {})
    }
}
