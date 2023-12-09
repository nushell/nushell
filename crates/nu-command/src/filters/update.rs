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
                description: "Use a closure to alter each value in the 'authors' column to a single string",
                example: "[[project, authors]; ['nu', ['Andrés', 'JT', 'Yehuda']]] | update authors {|row| $row.authors | str join ',' }",
                result: Some(Value::test_list(
                    vec![Value::test_record(record! {
                        "project" => Value::test_string("nu"),
                        "authors" => Value::test_string("Andrés,JT,Yehuda"),
                    })],
                )),
            },
            Example {
                description: "You can also use a simple command to update 'authors' to a single string",
                example: "[[project, authors]; ['nu', ['Andrés', 'JT', 'Yehuda']]] | update authors { str join ',' }",
                result: Some(Value::test_list(
                    vec![Value::test_record(record! {
                        "project" => Value::test_string("nu"),
                        "authors" => Value::test_string("Andrés,JT,Yehuda"),
                    })],
                )),
            },
            Example {
                description: "Update a value at an index in a list",
                example: "[1 2 3] | update 1 4",
                result: Some(Value::test_list(
                    vec![Value::test_int(1), Value::test_int(4), Value::test_int(3)]
                )),
            },
            Example {
                description: "Use a closure to compute a new value at an index",
                example: "[1 2 3] | update 1 {|i| $i + 2 }",
                result: Some(Value::test_list(
                    vec![Value::test_int(1), Value::test_int(4), Value::test_int(3)]
                )),
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
                            update_value_by_closure(
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
                        update_single_value_by_closure(
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
                value.update_data_at_cell_path(&cell_path.members, replacement)?;
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

                if replacement.as_block().is_ok() {
                    update_single_value_by_closure(
                        value,
                        replacement,
                        engine_state,
                        stack,
                        redirect_stdout,
                        redirect_stderr,
                        path,
                        true,
                    )?;
                } else {
                    value.update_data_at_cell_path(path, replacement)?;
                }

                Ok(pre_elems
                    .into_iter()
                    .chain(stream)
                    .into_pipeline_data_with_metadata(metadata, ctrlc))
            } else if replacement.as_block().is_ok() {
                let replacement_span = replacement.span();
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
    first_path_member_int: bool,
) -> Result<(), ShellError> {
    let input_at_path = value.clone().follow_cell_path(cell_path, false)?;

    if let Some(var) = block.signature.get_positional(0) {
        if let Some(var_id) = &var.var_id {
            stack.add_var(
                *var_id,
                if first_path_member_int {
                    input_at_path.clone()
                } else {
                    value.clone()
                },
            )
        }
    }

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

    update_value_by_closure(
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

        test_examples(Update {})
    }
}
