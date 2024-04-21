use nu_engine::{command_prelude::*, get_eval_block, EvalBlockFn};
use nu_protocol::{
    ast::{Block, PathMember},
    engine::Closure,
};

#[derive(Clone)]
pub struct Insert;

impl Command for Insert {
    fn name(&self) -> &str {
        "insert"
    }

    fn signature(&self) -> Signature {
        Signature::build("insert")
            .input_output_types(vec![
                (Type::Record(vec![]), Type::Record(vec![])),
                (Type::Table(vec![]), Type::Table(vec![])),
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

    fn usage(&self) -> &str {
        "Insert a new column, using an expression or closure to create each row's values."
    }

    fn extra_usage(&self) -> &str {
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
        ]
    }
}

fn insert(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;

    let cell_path: CellPath = call.req(engine_state, stack, 0)?;
    let replacement: Value = call.req(engine_state, stack, 1)?;
    let replacement_span = replacement.span();
    let ctrlc = engine_state.ctrlc.clone();

    let eval_block = get_eval_block(engine_state);

    match input {
        PipelineData::Value(mut value, metadata) => {
            if let Value::Closure { val: closure, .. } = replacement {
                match (cell_path.members.first(), &mut value) {
                    (Some(PathMember::String { .. }), Value::List { vals, .. }) => {
                        let block = engine_state.get_block(closure.block_id);
                        let stack = stack.captures_to_stack(closure.captures);
                        for val in vals {
                            let mut stack = stack.clone();
                            insert_value_by_closure(
                                val,
                                replacement_span,
                                engine_state,
                                &mut stack,
                                block,
                                &cell_path.members,
                                false,
                                eval_block,
                            )?;
                        }
                    }
                    (first, _) => {
                        insert_single_value_by_closure(
                            &mut value,
                            closure,
                            replacement_span,
                            engine_state,
                            stack,
                            &cell_path.members,
                            matches!(first, Some(PathMember::Int { .. })),
                            eval_block,
                        )?;
                    }
                }
            } else {
                value.insert_data_at_cell_path(&cell_path.members, replacement, span)?;
            }
            Ok(value.into_pipeline_data_with_metadata(metadata))
        }
        PipelineData::ListStream(mut stream, metadata) => {
            if let Some((
                &PathMember::Int {
                    val,
                    span: path_span,
                    ..
                },
                path,
            )) = cell_path.members.split_first()
            {
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
                    if let Value::Closure { val: closure, .. } = replacement {
                        let value = stream.next();
                        let end_of_stream = value.is_none();
                        let value = value.unwrap_or(Value::nothing(replacement_span));
                        let block = engine_state.get_block(closure.block_id);
                        let mut stack = stack.captures_to_stack(closure.captures);

                        if let Some(var) = block.signature.get_positional(0) {
                            if let Some(var_id) = &var.var_id {
                                stack.add_var(*var_id, value.clone())
                            }
                        }

                        let output = eval_block(
                            engine_state,
                            &mut stack,
                            block,
                            value.clone().into_pipeline_data(),
                        )?;

                        pre_elems.push(output.into_value(replacement_span));
                        if !end_of_stream {
                            pre_elems.push(value);
                        }
                    } else {
                        pre_elems.push(replacement);
                    }
                } else if let Some(mut value) = stream.next() {
                    if let Value::Closure { val: closure, .. } = replacement {
                        insert_single_value_by_closure(
                            &mut value,
                            closure,
                            replacement_span,
                            engine_state,
                            stack,
                            path,
                            true,
                            eval_block,
                        )?;
                    } else {
                        value.insert_data_at_cell_path(path, replacement, span)?;
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
                    .into_pipeline_data_with_metadata(metadata, ctrlc))
            } else if let Value::Closure { val: closure, .. } = replacement {
                let engine_state = engine_state.clone();
                let block = engine_state.get_block(closure.block_id).clone();
                let stack = stack.captures_to_stack(closure.captures);

                Ok(stream
                    .map(move |mut input| {
                        // Recreate the stack for each iteration to
                        // isolate environment variable changes, etc.
                        let mut stack = stack.clone();

                        let err = insert_value_by_closure(
                            &mut input,
                            replacement_span,
                            &engine_state,
                            &mut stack,
                            &block,
                            &cell_path.members,
                            false,
                            eval_block,
                        );

                        if let Err(e) = err {
                            Value::error(e, span)
                        } else {
                            input
                        }
                    })
                    .into_pipeline_data_with_metadata(metadata, ctrlc))
            } else {
                Ok(stream
                    .map(move |mut input| {
                        if let Err(e) = input.insert_data_at_cell_path(
                            &cell_path.members,
                            replacement.clone(),
                            span,
                        ) {
                            Value::error(e, span)
                        } else {
                            input
                        }
                    })
                    .into_pipeline_data_with_metadata(metadata, ctrlc))
            }
        }
        PipelineData::Empty => Err(ShellError::IncompatiblePathAccess {
            type_name: "empty pipeline".to_string(),
            span,
        }),
        PipelineData::ExternalStream { .. } => Err(ShellError::IncompatiblePathAccess {
            type_name: "external stream".to_string(),
            span,
        }),
    }
}

#[allow(clippy::too_many_arguments)]
fn insert_value_by_closure(
    value: &mut Value,
    span: Span,
    engine_state: &EngineState,
    stack: &mut Stack,
    block: &Block,
    cell_path: &[PathMember],
    first_path_member_int: bool,
    eval_block_fn: EvalBlockFn,
) -> Result<(), ShellError> {
    let input = if first_path_member_int {
        value
            .clone()
            .follow_cell_path(cell_path, false)
            .unwrap_or(Value::nothing(span))
    } else {
        value.clone()
    };

    if let Some(var) = block.signature.get_positional(0) {
        if let Some(var_id) = var.var_id {
            stack.add_var(var_id, input.clone());
        }
    }

    let output = eval_block_fn(engine_state, stack, block, input.into_pipeline_data())?;

    value.insert_data_at_cell_path(cell_path, output.into_value(span), span)
}

#[allow(clippy::too_many_arguments)]
fn insert_single_value_by_closure(
    value: &mut Value,
    closure: Closure,
    span: Span,
    engine_state: &EngineState,
    stack: &mut Stack,
    cell_path: &[PathMember],
    first_path_member_int: bool,
    eval_block_fn: EvalBlockFn,
) -> Result<(), ShellError> {
    let block = engine_state.get_block(closure.block_id);
    let mut stack = stack.captures_to_stack(closure.captures);

    insert_value_by_closure(
        value,
        span,
        engine_state,
        &mut stack,
        block,
        cell_path,
        first_path_member_int,
        eval_block_fn,
    )
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
