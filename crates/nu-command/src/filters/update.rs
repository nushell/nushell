use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::{Block, Call, CellPath, PathMember};
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    record, Category, Example, FromValue, IntoInterruptiblePipelineData, IntoPipelineData,
    PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Update;

impl Command for Update {
    fn name(&self) -> &str {
        "update"
    }

    fn signature(&self) -> Signature {
        Signature::build("update")
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
                "the name of the column to update",
            )
            .required(
                "replacement value",
                SyntaxShape::Any,
                "the new value to give the cell(s), or a closure to create the value",
            )
            .allow_variants_without_examples(true)
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Update an existing column to have a new value."
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

    fn examples(&self) -> Vec<Example> {
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
                description: "Use in closure form for more involved updating logic",
                example: "[[count fruit]; [1 'apple']] | enumerate | update item.count {|e| ($e.item.fruit | str length) + $e.index } | get item",
                result: Some(Value::test_list(
                    vec![Value::test_record(record! {
                        "count" => Value::test_int(5),
                        "fruit" => Value::test_string("apple"),
                    })],
                )),
            },
            Example {
                description: "Alter each value in the 'authors' column to use a single string instead of a list",
                example: "[[project, authors]; ['nu', ['Andrés', 'JT', 'Yehuda']]] | update authors {|row| $row.authors | str join ','}",
                result: Some(Value::test_list(
                    vec![Value::test_record(record! {
                        "project" => Value::test_string("nu"),
                        "authors" => Value::test_string("Andrés,JT,Yehuda"),
                    })],
                )),
            },
            Example {
                description: "You can also use a simple command to update 'authors' to a single string",
                example: "[[project, authors]; ['nu', ['Andrés', 'JT', 'Yehuda']]] | update authors {|| str join ','}",
                result: Some(Value::test_list(
                    vec![Value::test_record(record! {
                        "project" => Value::test_string("nu"),
                        "authors" => Value::test_string("Andrés,JT,Yehuda"),
                    })],
                )),
            }
        ]
    }
}

fn update(
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
                update_single_value_by_closure(
                    &mut value,
                    span,
                    replacement,
                    engine_state,
                    stack,
                    redirect_stdout,
                    redirect_stderr,
                    &cell_path.members,
                )?;
            } else {
                value.update_data_at_cell_path(&cell_path.members, replacement)?;
            }
            Ok(value.into_pipeline_data_with_metadata(metadata))
        }
        PipelineData::ListStream(mut stream, metadata) => {
            if let Some((&PathMember::Int { val, span, .. }, path)) =
                cell_path.members.split_first()
            {
                let mut pre_elems = vec![];

                for idx in 0..=val {
                    if let Some(v) = stream.next() {
                        pre_elems.push(v);
                    } else if idx == 0 {
                        return Err(ShellError::AccessEmptyContent { span });
                    } else {
                        return Err(ShellError::AccessBeyondEnd {
                            max_idx: idx - 1,
                            span,
                        });
                    }
                }

                // cannot fail since loop above does at least one iteration or returns an error
                let value = pre_elems.last_mut().unwrap();

                if replacement.as_block().is_ok() {
                    update_single_value_by_closure(
                        value,
                        span,
                        replacement,
                        engine_state,
                        stack,
                        redirect_stdout,
                        redirect_stderr,
                        path,
                    )?;
                } else {
                    value.update_data_at_cell_path(path, replacement)?;
                }

                Ok(pre_elems
                    .into_iter()
                    .chain(stream)
                    .into_pipeline_data_with_metadata(metadata, ctrlc))
            } else if replacement.as_block().is_ok() {
                let engine_state = engine_state.clone();
                let capture_block = Closure::from_value(replacement)?;
                let block = engine_state.get_block(capture_block.block_id).clone();
                let stack = stack.captures_to_stack(capture_block.captures.clone());

                Ok(stream
                    .map(move |mut input| {
                        // Recreate the stack for each iteration to
                        // isolate environment variable changes, etc.
                        let mut stack = stack.clone();

                        let err = update_value_by_closure(
                            &mut input,
                            span,
                            &engine_state,
                            &mut stack,
                            redirect_stdout,
                            redirect_stderr,
                            &block,
                            &cell_path.members,
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
                        if let Err(e) =
                            input.update_data_at_cell_path(&cell_path.members, replacement.clone())
                        {
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
fn update_value_by_closure(
    value: &mut Value,
    span: Span,
    engine_state: &EngineState,
    stack: &mut Stack,
    redirect_stdout: bool,
    redirect_stderr: bool,
    block: &Block,
    cell_path: &[PathMember],
) -> Result<(), ShellError> {
    if let Some(var) = block.signature.get_positional(0) {
        if let Some(var_id) = &var.var_id {
            stack.add_var(*var_id, value.clone())
        }
    }

    let input_at_path = value.clone().follow_cell_path(cell_path, false)?;

    let output = eval_block(
        engine_state,
        stack,
        block,
        input_at_path.into_pipeline_data(),
        redirect_stdout,
        redirect_stderr,
    )?;

    value.update_data_at_cell_path(cell_path, output.into_value(span))
}

#[allow(clippy::too_many_arguments)]
fn update_single_value_by_closure(
    value: &mut Value,
    span: Span,
    replacement: Value,
    engine_state: &EngineState,
    stack: &mut Stack,
    redirect_stdout: bool,
    redirect_stderr: bool,
    cell_path: &[PathMember],
) -> Result<(), ShellError> {
    let capture_block = Closure::from_value(replacement)?;
    let block = engine_state.get_block(capture_block.block_id).clone();
    let mut stack = stack.captures_to_stack(capture_block.captures);

    update_value_by_closure(
        value,
        span,
        engine_state,
        &mut stack,
        redirect_stdout,
        redirect_stderr,
        &block,
        cell_path,
    )
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
