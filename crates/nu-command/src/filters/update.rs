use nu_engine::{ClosureEval, ClosureEvalOnce, command_prelude::*};
use nu_protocol::ast::PathMember;

#[derive(Clone)]
pub struct Update;

impl Command for Update {
    fn name(&self) -> &str {
        "update"
    }

    fn signature(&self) -> Signature {
        Signature::build("update")
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
                "The name of the column to update.",
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
        "Update an existing column to have a new value."
    }

    fn extra_description(&self) -> &str {
        "When updating a column, the closure will be run for each row, and the current row will be passed as the first argument. \
Referencing `$in` inside the closure will provide the value at the column for the current row.

When updating a specific index, the closure will instead be run once. The first argument to the closure and the `$in` value will both be the current value at the index."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        update(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Update a column value",
                example: "{'name': 'nu', 'stars': 5} | update name 'Nushell'",
                result: Some(Value::test_record(record! {
                    "name" =>  Value::test_string("Nushell"),
                    "stars" => Value::test_int(5),
                })),
            },
            Example {
                description: "Use a closure to alter each value in the 'authors' column to a single string",
                example: "[[project, authors]; ['nu', ['Andrés', 'JT', 'Yehuda']]] | update authors {|row| $row.authors | str join ',' }",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "project" => Value::test_string("nu"),
                    "authors" => Value::test_string("Andrés,JT,Yehuda"),
                })])),
            },
            Example {
                description: "Implicitly use the `$in` value in a closure to update 'authors'",
                example: "[[project, authors]; ['nu', ['Andrés', 'JT', 'Yehuda']]] | update authors { str join ',' }",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "project" => Value::test_string("nu"),
                    "authors" => Value::test_string("Andrés,JT,Yehuda"),
                })])),
            },
            Example {
                description: "Update a value at an index in a list",
                example: "[1 2 3] | update 1 4",
                result: Some(Value::test_list(vec![
                    Value::test_int(1),
                    Value::test_int(4),
                    Value::test_int(3),
                ])),
            },
            Example {
                description: "Use a closure to compute a new value at an index",
                example: "[1 2 3] | update 1 {|i| $i + 2 }",
                result: Some(Value::test_list(vec![
                    Value::test_int(1),
                    Value::test_int(4),
                    Value::test_int(3),
                ])),
            },
            Example {
                description: "Use an optional cell path to update a column that may not exist in all rows",
                example: "[{a: 1, b: 2}, {b: 3}, {a: 4, b: 5}] | update a? 10",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "a" => Value::test_int(10),
                        "b" => Value::test_int(2),
                    }),
                    Value::test_record(record! {
                        "b" => Value::test_int(3),
                    }),
                    Value::test_record(record! {
                        "a" => Value::test_int(10),
                        "b" => Value::test_int(5),
                    }),
                ])),
            },
            Example {
                description: "Use an optional cell path to skip updating a value that doesn't exist",
                example: "{a: 1} | update b? 2",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(1),
                })),
            },
        ]
    }
}

fn update(
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
                            update_value_by_closure(
                                val,
                                &mut closure,
                                head,
                                &cell_path.members,
                                false,
                            )?;
                        }
                    }
                    (first, _) => {
                        update_single_value_by_closure(
                            &mut value,
                            ClosureEvalOnce::new(engine_state, stack, *val),
                            head,
                            &cell_path.members,
                            matches!(first, Some(PathMember::Int { .. })),
                        )?;
                    }
                }
            } else {
                // Check if the path is optional
                let is_optional = is_cell_path_optional(&cell_path.members);

                // For tables we need to handle optional paths specially
                if is_optional {
                    match (&cell_path.members.first(), &mut value) {
                        (Some(PathMember::String { .. }), Value::List { vals, .. }) => {
                            // For each record in the table, only update if the path exists
                            for val in vals {
                                if let Ok(value_at_path) = val.follow_cell_path(&cell_path.members)
                                {
                                    if !matches!(value_at_path.as_ref(), Value::Nothing { .. }) {
                                        // Path exists, do the update
                                        val.update_data_at_cell_path(
                                            &cell_path.members,
                                            replacement.clone(),
                                        )?;
                                    }
                                }
                            }
                        }
                        _ => {
                            // For single values, check if path exists
                            if let Ok(value_at_path) = value.follow_cell_path(&cell_path.members) {
                                if !matches!(value_at_path.as_ref(), Value::Nothing { .. }) {
                                    // Path exists, do the update
                                    value.update_data_at_cell_path(
                                        &cell_path.members,
                                        replacement,
                                    )?;
                                }
                            }
                        }
                    }
                } else {
                    // Path is not optional
                    value.update_data_at_cell_path(&cell_path.members, replacement)?;
                }
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

                for idx in 0..=val {
                    if let Some(v) = stream.next() {
                        pre_elems.push(v);
                    } else if idx == 0 {
                        return Err(ShellError::AccessEmptyContent { span: path_span });
                    } else {
                        return Err(ShellError::AccessBeyondEnd {
                            max_idx: idx - 1,
                            span: path_span,
                        });
                    }
                }

                // cannot fail since loop above does at least one iteration or returns an error
                let value = pre_elems.last_mut().expect("one element");

                if let Value::Closure { val, .. } = replacement {
                    update_single_value_by_closure(
                        value,
                        ClosureEvalOnce::new(engine_state, stack, *val),
                        head,
                        path,
                        true,
                    )?;
                } else {
                    // Check if the path is optional and doesn't exist
                    if is_cell_path_optional(path) {
                        if let Ok(value_at_path) = value.follow_cell_path(path) {
                            if matches!(value_at_path.as_ref(), Value::Nothing { .. }) {
                                return Ok(pre_elems
                                    .into_iter()
                                    .chain(stream)
                                    .into_pipeline_data_with_metadata(
                                        head,
                                        engine_state.signals().clone(),
                                        metadata,
                                    ));
                            }
                        }
                    }
                    value.update_data_at_cell_path(path, replacement)?;
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
                    let err = update_value_by_closure(
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
                let is_optional = is_cell_path_optional(&cell_path.members);
                let stream = stream.map(move |mut value| {
                    // Check if the path is optional and doesn't exist
                    if is_optional {
                        if let Ok(value_at_path) = value.follow_cell_path(&cell_path.members) {
                            if matches!(value_at_path.as_ref(), Value::Nothing { .. }) {
                                return value;
                            }
                        }
                    }

                    if let Err(e) =
                        value.update_data_at_cell_path(&cell_path.members, replacement.clone())
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

fn is_cell_path_optional(cell_path: &[PathMember]) -> bool {
    cell_path.iter().any(|member| match member {
        PathMember::String { optional, .. } => *optional,
        PathMember::Int { optional, .. } => *optional,
    })
}

fn update_value_by_closure(
    value: &mut Value,
    closure: &mut ClosureEval,
    span: Span,
    cell_path: &[PathMember],
    first_path_member_int: bool,
) -> Result<(), ShellError> {
    let value_at_path = value.follow_cell_path(cell_path)?;

    // If the cell path is optional and the value doesn't exist, skip the update
    if is_cell_path_optional(cell_path) && matches!(value_at_path.as_ref(), Value::Nothing { .. }) {
        return Ok(());
    }

    let arg = if first_path_member_int {
        value_at_path.as_ref()
    } else {
        &*value
    };

    let new_value = closure
        .add_arg(arg.clone())
        .run_with_input(value_at_path.into_owned().into_pipeline_data())?
        .into_value(span)?;

    value.update_data_at_cell_path(cell_path, new_value)
}

fn update_single_value_by_closure(
    value: &mut Value,
    closure: ClosureEvalOnce,
    span: Span,
    cell_path: &[PathMember],
    first_path_member_int: bool,
) -> Result<(), ShellError> {
    let value_at_path = value.follow_cell_path(cell_path)?;

    // If the cell path is optional and the value doesn't exist, skip the update
    if is_cell_path_optional(cell_path) && matches!(value_at_path.as_ref(), Value::Nothing { .. }) {
        return Ok(());
    }

    let arg = if first_path_member_int {
        value_at_path.as_ref()
    } else {
        &*value
    };

    let new_value = closure
        .add_arg(arg.clone())
        .run_with_input(value_at_path.into_owned().into_pipeline_data())?
        .into_value(span)?;

    value.update_data_at_cell_path(cell_path, new_value)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Update {})
    }
}
