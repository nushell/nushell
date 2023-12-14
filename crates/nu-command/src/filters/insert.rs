use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::{Block, Call, CellPath, PathMember};
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    record, Category, Example, FromValue, IntoInterruptiblePipelineData, IntoPipelineData,
    PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
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
                "the name of the column to insert",
            )
            .required(
                "new value",
                SyntaxShape::Any,
                "the new value to give the cell(s)",
            )
            .allow_variants_without_examples(true)
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Insert a new column, using an expression or closure to create each row's values."
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
                    "name" =>  Value::test_string("nu"),
                    "stars" => Value::test_int(5),
                    "alias" => Value::test_string("Nushell"),
                })),
            },
            Example {
                description: "Insert a new column into a table, populating all rows",
                example: "[[project, lang]; ['Nushell', 'Rust']] | insert type 'shell'",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "project" => Value::test_string("Nushell"),
                    "lang" =>    Value::test_string("Rust"),
                    "type" =>    Value::test_string("shell"),
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

    let redirect_stdout = call.redirect_stdout;
    let redirect_stderr = call.redirect_stderr;

    let ctrlc = engine_state.ctrlc.clone();

    match input {
        PipelineData::Value(mut value, metadata) => {
            if replacement.as_block().is_ok() {
                match (cell_path.members.first(), &mut value) {
                    (Some(PathMember::String { .. }), Value::List { vals, .. }) => {
                        let span = replacement.span();
                        let capture_block = Closure::from_value(replacement)?;
                        let block = engine_state.get_block(capture_block.block_id);
                        let stack = stack.captures_to_stack(capture_block.captures.clone());
                        for val in vals {
                            let mut stack = stack.clone();
                            insert_value_by_closure(
                                val,
                                span,
                                engine_state,
                                &mut stack,
                                redirect_stdout,
                                redirect_stderr,
                                block,
                                &cell_path.members,
                                false,
                            )?;
                        }
                    }
                    (first, _) => {
                        insert_single_value_by_closure(
                            &mut value,
                            replacement,
                            engine_state,
                            stack,
                            redirect_stdout,
                            redirect_stderr,
                            &cell_path.members,
                            matches!(first, Some(PathMember::Int { .. })),
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
                    if replacement.as_block().is_ok() {
                        let span = replacement.span();
                        let value = stream.next();
                        let end_of_stream = value.is_none();
                        let value = value.unwrap_or(Value::nothing(span));
                        let capture_block = Closure::from_value(replacement)?;
                        let block = engine_state.get_block(capture_block.block_id);
                        let mut stack = stack.captures_to_stack(capture_block.captures);

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
                            redirect_stdout,
                            redirect_stderr,
                        )?;

                        pre_elems.push(output.into_value(span));
                        if !end_of_stream {
                            pre_elems.push(value);
                        }
                    } else {
                        pre_elems.push(replacement);
                    }
                } else if let Some(mut value) = stream.next() {
                    if replacement.as_block().is_ok() {
                        insert_single_value_by_closure(
                            &mut value,
                            replacement,
                            engine_state,
                            stack,
                            redirect_stdout,
                            redirect_stderr,
                            path,
                            true,
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
            } else if replacement.as_block().is_ok() {
                let engine_state = engine_state.clone();
                let replacement_span = replacement.span();
                let capture_block = Closure::from_value(replacement)?;
                let block = engine_state.get_block(capture_block.block_id).clone();
                let stack = stack.captures_to_stack(capture_block.captures.clone());

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
                            redirect_stdout,
                            redirect_stderr,
                            &block,
                            &cell_path.members,
                            false,
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
    redirect_stdout: bool,
    redirect_stderr: bool,
    block: &Block,
    cell_path: &[PathMember],
    first_path_member_int: bool,
) -> Result<(), ShellError> {
    let input_at_path = value.clone().follow_cell_path(cell_path, false);

    if let Some(var) = block.signature.get_positional(0) {
        if let Some(var_id) = &var.var_id {
            stack.add_var(
                *var_id,
                if first_path_member_int {
                    input_at_path.clone().unwrap_or(Value::nothing(span))
                } else {
                    value.clone()
                },
            )
        }
    }

    let input_at_path = input_at_path
        .map(IntoPipelineData::into_pipeline_data)
        .unwrap_or(PipelineData::Empty);

    let output = eval_block(
        engine_state,
        stack,
        block,
        input_at_path,
        redirect_stdout,
        redirect_stderr,
    )?;

    value.insert_data_at_cell_path(cell_path, output.into_value(span), span)
}

#[allow(clippy::too_many_arguments)]
fn insert_single_value_by_closure(
    value: &mut Value,
    replacement: Value,
    engine_state: &EngineState,
    stack: &mut Stack,
    redirect_stdout: bool,
    redirect_stderr: bool,
    cell_path: &[PathMember],
    first_path_member_int: bool,
) -> Result<(), ShellError> {
    let span = replacement.span();
    let capture_block = Closure::from_value(replacement)?;
    let block = engine_state.get_block(capture_block.block_id);
    let mut stack = stack.captures_to_stack(capture_block.captures);

    insert_value_by_closure(
        value,
        span,
        engine_state,
        &mut stack,
        redirect_stdout,
        redirect_stderr,
        block,
        cell_path,
        first_path_member_int,
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
