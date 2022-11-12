use super::utils::chain_error_with_input;
use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, ShellError,
    Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Where;

impl Command for Where {
    fn name(&self) -> &str {
        "where"
    }

    fn usage(&self) -> &str {
        "Filter values based on a condition."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("where")
            .input_output_types(vec![
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
                (Type::Table(vec![]), Type::Table(vec![])),
            ])
            .optional("cond", SyntaxShape::RowCondition, "condition")
            .named(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "use where with a closure",
                Some('b'),
            )
            .category(Category::Filters)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["filter", "find", "search", "condition"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        if let Ok(Some(capture_block)) = call.get_flag::<Closure>(engine_state, stack, "block") {
            let metadata = input.metadata();
            let ctrlc = engine_state.ctrlc.clone();
            let engine_state = engine_state.clone();
            let block = engine_state.get_block(capture_block.block_id).clone();
            let mut stack = stack.captures_to_stack(&capture_block.captures);
            let orig_env_vars = stack.env_vars.clone();
            let orig_env_hidden = stack.env_hidden.clone();
            let span = call.head;
            let redirect_stdout = call.redirect_stdout;
            let redirect_stderr = call.redirect_stderr;

            match input {
                PipelineData::Value(Value::Range { .. }, ..)
                | PipelineData::Value(Value::List { .. }, ..)
                | PipelineData::ListStream { .. } => Ok(input
                    .into_iter()
                    .filter_map(move |x| {
                        // with_env() is used here to ensure that each iteration uses
                        // a different set of environment variables.
                        // Hence, a 'cd' in the first loop won't affect the next loop.
                        stack.with_env(&orig_env_vars, &orig_env_hidden);

                        if let Some(var) = block.signature.get_positional(0) {
                            if let Some(var_id) = &var.var_id {
                                stack.add_var(*var_id, x.clone());
                            }
                        }

                        match eval_block(
                            &engine_state,
                            &mut stack,
                            &block,
                            // clone() is used here because x is given to Ok() below.
                            x.clone().into_pipeline_data(),
                            redirect_stdout,
                            redirect_stderr,
                        ) {
                            Ok(v) => {
                                if v.into_value(span).is_true() {
                                    Some(x)
                                } else {
                                    None
                                }
                            }
                            Err(error) => Some(Value::Error {
                                error: chain_error_with_input(error, x.span()),
                            }),
                        }
                    })
                    .into_pipeline_data(ctrlc)),
                PipelineData::ExternalStream { stdout: None, .. } => {
                    Ok(PipelineData::new(call.head))
                }
                PipelineData::ExternalStream {
                    stdout: Some(stream),
                    ..
                } => Ok(stream
                    .into_iter()
                    .filter_map(move |x| {
                        // see note above about with_env()
                        stack.with_env(&orig_env_vars, &orig_env_hidden);

                        let x = match x {
                            Ok(x) => x,
                            Err(err) => return Some(Value::Error { error: err }),
                        };

                        if let Some(var) = block.signature.get_positional(0) {
                            if let Some(var_id) = &var.var_id {
                                stack.add_var(*var_id, x.clone());
                            }
                        }

                        match eval_block(
                            &engine_state,
                            &mut stack,
                            &block,
                            // clone() is used here because x is given to Ok() below.
                            x.clone().into_pipeline_data(),
                            redirect_stdout,
                            redirect_stderr,
                        ) {
                            Ok(v) => {
                                if v.into_value(span).is_true() {
                                    Some(x)
                                } else {
                                    None
                                }
                            }
                            Err(error) => Some(Value::Error {
                                error: chain_error_with_input(error, x.span()),
                            }),
                        }
                    })
                    .into_pipeline_data(ctrlc)),
                PipelineData::Value(x, ..) => {
                    // see note above about with_env()
                    stack.with_env(&orig_env_vars, &orig_env_hidden);

                    if let Some(var) = block.signature.get_positional(0) {
                        if let Some(var_id) = &var.var_id {
                            stack.add_var(*var_id, x.clone());
                        }
                    }
                    Ok(match eval_block(
                        &engine_state,
                        &mut stack,
                        &block,
                        // clone() is used here because x is given to Ok() below.
                        x.clone().into_pipeline_data(),
                        redirect_stdout,
                        redirect_stderr,
                    ) {
                        Ok(v) => {
                            if v.into_value(span).is_true() {
                                Some(x)
                            } else {
                                None
                            }
                        }
                        Err(error) => Some(Value::Error {
                            error: chain_error_with_input(error, x.span()),
                        }),
                    }
                    .into_pipeline_data(ctrlc))
                }
            }
            .map(|x| x.set_metadata(metadata))
        } else {
            let capture_block: Option<Closure> = call.opt(engine_state, stack, 0)?;
            if let Some(block) = capture_block {
                let span = call.head;

                let metadata = input.metadata();
                let mut stack = stack.captures_to_stack(&block.captures);
                let block = engine_state.get_block(block.block_id).clone();

                let orig_env_vars = stack.env_vars.clone();
                let orig_env_hidden = stack.env_hidden.clone();

                let ctrlc = engine_state.ctrlc.clone();
                let engine_state = engine_state.clone();

                let redirect_stdout = call.redirect_stdout;
                let redirect_stderr = call.redirect_stderr;
                Ok(input
                    .into_iter()
                    .filter_map(move |value| {
                        stack.with_env(&orig_env_vars, &orig_env_hidden);

                        if let Some(var) = block.signature.get_positional(0) {
                            if let Some(var_id) = &var.var_id {
                                stack.add_var(*var_id, value.clone());
                            }
                        }
                        let result = eval_block(
                            &engine_state,
                            &mut stack,
                            &block,
                            // clone() is used here because x is given to Ok() below.
                            value.clone().into_pipeline_data(),
                            redirect_stdout,
                            redirect_stderr,
                        );

                        match result {
                            Ok(result) => {
                                let result = result.into_value(span);
                                if result.is_true() {
                                    Some(value)
                                } else {
                                    None
                                }
                            }
                            Err(err) => Some(Value::Error { error: err }),
                        }
                    })
                    .into_pipeline_data(ctrlc))
                .map(|x| x.set_metadata(metadata))
            } else {
                Err(ShellError::MissingParameter(
                    "condition".to_string(),
                    call.head,
                ))
            }
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Filter rows of a table according to a condition",
                example: "[{a: 1} {a: 2}] | where a > 1",
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec!["a".to_string()],
                        vals: vec![Value::test_int(2)],
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Filter items of a list according to a condition",
                example: "[1 2] | where {|x| $x > 1}",
                result: Some(Value::List {
                    vals: vec![Value::test_int(2)],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "List all files in the current directory with sizes greater than 2kb",
                example: "ls | where size > 2kb",
                result: None,
            },
            Example {
                description: "List only the files in the current directory",
                example: "ls | where type == file",
                result: None,
            },
            Example {
                description: "List all files with names that contain \"Car\"",
                example: "ls | where name =~ \"Car\"",
                result: None,
            },
            Example {
                description: "List all files that were modified in the last two weeks",
                example: "ls | where modified >= (date now) - 2wk",
                result: None,
            },
            // TODO: This should work but does not. (Note that `Let` must be present in the working_set in `example_test.rs`).
            // See https://github.com/nushell/nushell/issues/7034
            // Example {
            //     description: "Get all numbers above 3 with an existing block condition",
            //     example: "let a = {$in > 3}; [1, 2, 5, 6] | where -b $a",
            //     result: Some(Value::List {
            //         vals: vec![
            //             Value::Int {
            //                 val: 5,
            //                 span: Span::test_data(),
            //             },
            //             Value::Int {
            //                 val: 6,
            //                 span: Span::test_data(),
            //             },
            //         ],
            //         span: Span::test_data(),
            //     }),
            // },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Where {})
    }
}
