use std::borrow::Cow;

use nu_engine::{ClosureEval, ClosureEvalOnce, command_prelude::*};
use nu_protocol::ast::PathMember;

#[derive(Clone)]
pub struct Upsert;

impl Command for Upsert {
    fn name(&self) -> &str {
        "upsert"
    }

    fn signature(&self) -> Signature {
        Signature::build("upsert")
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
                "The name of the column to update or insert.",
            )
            .required(
                "replacement value",
                SyntaxShape::Any,
                "The new value to give the cell(s), or a closure to create the value.",
            )
            .allow_variants_without_examples(true)
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Update an existing column to have a new value, or insert a new column."
    }

    fn extra_description(&self) -> &str {
        "When updating or inserting a column, the closure will be run for each row, and the current row will be passed as the first argument. \
Referencing `$in` inside the closure will provide the value at the column for the current row or null if the column does not exist.

When updating a specific index, the closure will instead be run once. The first argument to the closure and the `$in` value will both be the current value at the index. \
If the command is inserting at the end of a list or table, then both of these values will be null."
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
        upsert(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Update a record's value",
                example: "{'name': 'nu', 'stars': 5} | upsert name 'Nushell'",
                result: Some(Value::test_record(record! {
                    "name" => Value::test_string("Nushell"),
                    "stars" => Value::test_int(5),
                })),
            },
            Example {
                description: "Insert a new entry into a record",
                example: "{'name': 'nu', 'stars': 5} | upsert language 'Rust'",
                result: Some(Value::test_record(record! {
                    "name" =>     Value::test_string("nu"),
                    "stars" =>    Value::test_int(5),
                    "language" => Value::test_string("Rust"),
                })),
            },
            Example {
                description: "Update each row of a table",
                example: "[[name lang]; [Nushell ''] [Reedline '']] | upsert lang 'Rust'",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "name" => Value::test_string("Nushell"),
                        "lang" => Value::test_string("Rust"),
                    }),
                    Value::test_record(record! {
                        "name" => Value::test_string("Reedline"),
                        "lang" => Value::test_string("Rust"),
                    }),
                ])),
            },
            Example {
                description: "Insert a new column with values computed based off the other columns",
                example: "[[foo]; [7] [8] [9]] | upsert bar {|row| $row.foo * 2 }",
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
                description: "Update null values in a column to a default value",
                example: "[[foo]; [2] [null] [4]] | upsert foo { default 0 }",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "foo" => Value::test_int(2),
                    }),
                    Value::test_record(record! {
                        "foo" => Value::test_int(0),
                    }),
                    Value::test_record(record! {
                        "foo" => Value::test_int(4),
                    }),
                ])),
            },
            Example {
                description: "Upsert into a list, updating an existing value at an index",
                example: "[1 2 3] | upsert 0 2",
                result: Some(Value::test_list(vec![
                    Value::test_int(2),
                    Value::test_int(2),
                    Value::test_int(3),
                ])),
            },
            Example {
                description: "Upsert into a list, inserting a new value at the end",
                example: "[1 2 3] | upsert 3 4",
                result: Some(Value::test_list(vec![
                    Value::test_int(1),
                    Value::test_int(2),
                    Value::test_int(3),
                    Value::test_int(4),
                ])),
            },
            Example {
                description: "Upsert into a nested path, creating new values as needed",
                example: "[{} {a: [{}]}] | upsert a.0.b \"value\"",
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

fn upsert(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let cell_path: CellPath = call.req(engine_state, stack, 0)?;
    let replacement: Value = call.req(engine_state, stack, 1)?;

    match input {
        PipelineData::Value(mut value, metadata) => {
            if let Value::Closure { val, .. } = replacement {
                match (cell_path.members.first(), &mut value) {
                    (Some(PathMember::String { .. }), Value::List { vals, .. }) => {
                        let mut closure = ClosureEval::new(engine_state, stack, *val);
                        for val in vals {
                            upsert_value_by_closure(
                                val,
                                &mut closure,
                                head,
                                &cell_path.members,
                                false,
                            )?;
                        }
                    }
                    (first, _) => {
                        upsert_single_value_by_closure(
                            &mut value,
                            ClosureEvalOnce::new(engine_state, stack, *val),
                            head,
                            &cell_path.members,
                            matches!(first, Some(PathMember::Int { .. })),
                        )?;
                    }
                }
            } else {
                value.upsert_data_at_cell_path(&cell_path.members, replacement)?;
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

                let value = if path.is_empty() {
                    let value = stream.next().unwrap_or(Value::nothing(head));
                    if let Value::Closure { val, .. } = replacement {
                        ClosureEvalOnce::new(engine_state, stack, *val)
                            .run_with_value(value)?
                            .into_value(head)?
                    } else {
                        replacement
                    }
                } else if let Some(mut value) = stream.next() {
                    if let Value::Closure { val, .. } = replacement {
                        upsert_single_value_by_closure(
                            &mut value,
                            ClosureEvalOnce::new(engine_state, stack, *val),
                            head,
                            path,
                            true,
                        )?;
                    } else {
                        value.upsert_data_at_cell_path(path, replacement)?;
                    }
                    value
                } else {
                    return Err(ShellError::AccessBeyondEnd {
                        max_idx: pre_elems.len() - 1,
                        span: path_span,
                    });
                };

                pre_elems.push(value);

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
                    let err = upsert_value_by_closure(
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
                    if let Err(e) =
                        value.upsert_data_at_cell_path(&cell_path.members, replacement.clone())
                    {
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

fn upsert_value_by_closure(
    value: &mut Value,
    closure: &mut ClosureEval,
    span: Span,
    cell_path: &[PathMember],
    first_path_member_int: bool,
) -> Result<(), ShellError> {
    let value_at_path = value.follow_cell_path(cell_path);

    let arg = if first_path_member_int {
        value_at_path
            .as_deref()
            .cloned()
            .unwrap_or(Value::nothing(span))
    } else {
        value.clone()
    };

    let input = value_at_path
        .map(Cow::into_owned)
        .map(IntoPipelineData::into_pipeline_data)
        .unwrap_or(PipelineData::empty());

    let new_value = closure
        .add_arg(arg)
        .run_with_input(input)?
        .into_value(span)?;

    value.upsert_data_at_cell_path(cell_path, new_value)
}

fn upsert_single_value_by_closure(
    value: &mut Value,
    closure: ClosureEvalOnce,
    span: Span,
    cell_path: &[PathMember],
    first_path_member_int: bool,
) -> Result<(), ShellError> {
    let value_at_path = value.follow_cell_path(cell_path);

    let arg = if first_path_member_int {
        value_at_path
            .as_deref()
            .cloned()
            .unwrap_or(Value::nothing(span))
    } else {
        value.clone()
    };

    let input = value_at_path
        .map(Cow::into_owned)
        .map(IntoPipelineData::into_pipeline_data)
        .unwrap_or(PipelineData::empty());

    let new_value = closure
        .add_arg(arg)
        .run_with_input(input)?
        .into_value(span)?;

    value.upsert_data_at_cell_path(cell_path, new_value)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Upsert {})
    }
}
