use nu_engine::{eval_block_with_early_return, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, Signature,
    Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct EachWhile;

impl Command for EachWhile {
    fn name(&self) -> &str {
        "each while"
    }

    fn usage(&self) -> &str {
        "Run a block on each row of the input list until a null is found, then create a new list with the results."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["for", "loop", "iterate"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build(self.name())
            .input_output_types(vec![(
                Type::List(Box::new(Type::Any)),
                Type::List(Box::new(Type::Any)),
            )])
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any, SyntaxShape::Int])),
                "the closure to run",
            )
            .switch(
                "numbered",
                "iterate with an index (deprecated; use a two-parameter closure instead)",
                Some('n'),
            )
            .category(Category::Filters)
    }

    fn examples(&self) -> Vec<Example> {
        let stream_test_1 = vec![
            Value::int(2, Span::test_data()),
            Value::int(4, Span::test_data()),
        ];
        let stream_test_2 = vec![
            Value::string("Output: 1", Span::test_data()),
            Value::string("Output: 2", Span::test_data()),
        ];

        vec![
            Example {
                example: "[1 2 3 2 1] | each while {|e| if $e < 3 { $e * 2 } }",
                description: "Produces a list of each element before the 3, doubled",
                result: Some(Value::List {
                    vals: stream_test_1,
                    span: Span::test_data(),
                }),
            },
            Example {
                example: r#"[1 2 stop 3 4] | each while {|e| if $e != 'stop' { $"Output: ($e)" } }"#,
                description: "Output elements until reaching 'stop'",
                result: Some(Value::List {
                    vals: stream_test_2,
                    span: Span::test_data(),
                }),
            },
            Example {
                example: r#"[1 2 3] | each while {|el ind| if $el < 2 { $"value ($el) at ($ind)!"} }"#,
                description: "Iterate over each element, printing the matching value and its index",
                result: Some(Value::List {
                    vals: vec![Value::string("value 1 at 0!", Span::test_data())],
                    span: Span::test_data(),
                }),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let capture_block: Closure = call.req(engine_state, stack, 0)?;
        let numbered = call.has_flag("numbered");

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
                // TODO: Could this be changed to .into_interruptible_iter(ctrlc) ?
                .into_iter()
                .enumerate()
                .map_while(move |(idx, x)| {
                    // with_env() is used here to ensure that each iteration uses
                    // a different set of environment variables.
                    // Hence, a 'cd' in the first loop won't affect the next loop.
                    stack.with_env(&orig_env_vars, &orig_env_hidden);

                    if let Some(var) = block.signature.get_positional(0) {
                        if let Some(var_id) = &var.var_id {
                            if numbered {
                                stack.add_var(
                                    *var_id,
                                    Value::Record {
                                        cols: vec!["index".into(), "item".into()],
                                        vals: vec![
                                            Value::Int {
                                                val: idx as i64,
                                                span,
                                            },
                                            x.clone(),
                                        ],
                                        span,
                                    },
                                );
                            } else {
                                stack.add_var(*var_id, x.clone());
                            }
                        }
                    }
                    // Optional second index argument
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

                    match eval_block_with_early_return(
                        &engine_state,
                        &mut stack,
                        &block,
                        x.into_pipeline_data(),
                        redirect_stdout,
                        redirect_stderr,
                    ) {
                        Ok(v) => {
                            let value = v.into_value(span);
                            if value.is_nothing() {
                                None
                            } else {
                                Some(value)
                            }
                        }
                        Err(_) => None,
                    }
                })
                .fuse()
                .into_pipeline_data(ctrlc)),
            PipelineData::ExternalStream { stdout: None, .. } => Ok(PipelineData::empty()),
            PipelineData::ExternalStream {
                stdout: Some(stream),
                ..
            } => Ok(stream
                .into_iter()
                .enumerate()
                .map_while(move |(idx, x)| {
                    // with_env() is used here to ensure that each iteration uses
                    // a different set of environment variables.
                    // Hence, a 'cd' in the first loop won't affect the next loop.
                    stack.with_env(&orig_env_vars, &orig_env_hidden);

                    let x = match x {
                        Ok(x) => x,
                        Err(_) => return None,
                    };

                    if let Some(var) = block.signature.get_positional(0) {
                        if let Some(var_id) = &var.var_id {
                            if numbered {
                                stack.add_var(
                                    *var_id,
                                    Value::Record {
                                        cols: vec!["index".into(), "item".into()],
                                        vals: vec![
                                            Value::Int {
                                                val: idx as i64,
                                                span,
                                            },
                                            x.clone(),
                                        ],
                                        span,
                                    },
                                );
                            } else {
                                stack.add_var(*var_id, x.clone());
                            }
                        }
                    }

                    match eval_block_with_early_return(
                        &engine_state,
                        &mut stack,
                        &block,
                        x.into_pipeline_data(),
                        redirect_stdout,
                        redirect_stderr,
                    ) {
                        Ok(v) => {
                            let value = v.into_value(span);
                            if value.is_nothing() {
                                None
                            } else {
                                Some(value)
                            }
                        }
                        Err(_) => None,
                    }
                })
                .fuse()
                .into_pipeline_data(ctrlc)),
            // This match allows non-iterables to be accepted,
            // which is currently considered undesirable (Nov 2022).
            PipelineData::Value(x, ..) => {
                if let Some(var) = block.signature.get_positional(0) {
                    if let Some(var_id) = &var.var_id {
                        stack.add_var(*var_id, x.clone());
                    }
                }

                eval_block_with_early_return(
                    &engine_state,
                    &mut stack,
                    &block,
                    x.into_pipeline_data(),
                    redirect_stdout,
                    redirect_stderr,
                )
            }
        }
        .map(|x| x.set_metadata(metadata))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use nu_test_support::{nu, pipeline};

    #[test]
    fn uses_optional_index_argument() {
        let actual = nu!(
            cwd: ".", pipeline(
            r#"[7 8 9 10] | each while {|el ind| $el + $ind } | to nuon"#
        ));

        assert_eq!(actual.out, "[7, 9, 11, 13]");
    }
    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(EachWhile {})
    }
}
