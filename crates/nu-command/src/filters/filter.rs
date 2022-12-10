use super::utils::chain_error_with_input;
use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, Signature,
    Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Filter;

impl Command for Filter {
    fn name(&self) -> &str {
        "filter"
    }

    fn usage(&self) -> &str {
        "Filter values based on a predicate closure."
    }

    fn extra_usage(&self) -> &str {
        r#"This command works similar to 'where' but allows reading the predicate closure from
a variable. On the other hand, the "row condition" syntax is not supported."#
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("filter")
            .input_output_types(vec![
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
                (Type::Table(vec![]), Type::Table(vec![])),
            ])
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any, SyntaxShape::Int])),
                "Predicate closure",
            )
            .category(Category::Filters)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["where", "find", "search", "condition"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let capture_block: Closure = call.req(engine_state, stack, 0)?;
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
            PipelineData::Empty => Ok(PipelineData::Empty),
            PipelineData::Value(Value::Range { .. }, ..)
            | PipelineData::Value(Value::List { .. }, ..)
            | PipelineData::ListStream { .. } => Ok(input
                // To enumerate over the input (for the index argument),
                // it must be converted into an iterator using into_iter().
                .into_iter()
                .enumerate()
                .filter_map(move |(idx, x)| {
                    // with_env() is used here to ensure that each iteration uses
                    // a different set of environment variables.
                    // Hence, a 'cd' in the first loop won't affect the next loop.
                    stack.with_env(&orig_env_vars, &orig_env_hidden);

                    if let Some(var) = block.signature.get_positional(0) {
                        if let Some(var_id) = &var.var_id {
                            stack.add_var(*var_id, x.clone());
                        }
                    }
                    // Optional index argument
                    if let Some(var) = block.signature.get_positional(1) {
                        if let Some(var_id) = &var.var_id {
                            stack.add_var(
                                *var_id,
                                Value::Int {
                                    val: idx as i64,
                                    span,
                                },
                            );
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
            PipelineData::ExternalStream { stdout: None, .. } => Ok(PipelineData::empty()),
            PipelineData::ExternalStream {
                stdout: Some(stream),
                ..
            } => Ok(stream
                .into_iter()
                .enumerate()
                .filter_map(move |(idx, x)| {
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
                    // Optional index argument
                    if let Some(var) = block.signature.get_positional(1) {
                        if let Some(var_id) = &var.var_id {
                            stack.add_var(
                                *var_id,
                                Value::Int {
                                    val: idx as i64,
                                    span,
                                },
                            );
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
            // This match allows non-iterables to be accepted,
            // which is currently considered undesirable (Nov 2022).
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
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Filter items of a list according to a condition",
                example: "[1 2] | filter {|x| $x > 1}",
                result: Some(Value::List {
                    vals: vec![Value::test_int(2)],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Filter rows of a table according to a condition",
                example: "[{a: 1} {a: 2}] | filter {|x| $x.a > 1}",
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
                description: "Filter rows of a table according to a stored condition",
                example: "let cond = {|x| $x.a > 1}; [{a: 1} {a: 2}] | filter $cond",
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec!["a".to_string()],
                        vals: vec![Value::test_int(2)],
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
            // TODO: This should work but does not. (Note that `Let` must be present in the working_set in `example_test.rs`).
            // See https://github.com/nushell/nushell/issues/7034
            // Example {
            //     description: "List all numbers above 3, using an existing closure condition",
            //     example: "let a = {$in > 3}; [1, 2, 5, 6] | filter $a",
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

        test_examples(Filter {})
    }
}
