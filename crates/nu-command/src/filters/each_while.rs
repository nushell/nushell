use nu_engine::{eval_block, CallExt};
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
        "Run a block on each element of input until a null is found"
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
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "the closure to run",
            )
            .switch("numbered", "iterate with an index", Some('n'))
            .category(Category::Filters)
    }

    fn examples(&self) -> Vec<Example> {
        let stream_test_1 = vec![
            Value::Int {
                val: 2,
                span: Span::test_data(),
            },
            Value::Int {
                val: 4,
                span: Span::test_data(),
            },
        ];
        let stream_test_2 = vec![
            Value::String {
                val: "Output: 1".into(),
                span: Span::test_data(),
            },
            Value::String {
                val: "Output: 2".into(),
                span: Span::test_data(),
            },
        ];

        vec![
            Example {
                example: "[1 2 3] | each while { |it| if $it < 3 { $it * 2 } else { null } }",
                description: "Multiplies elements below three by two",
                result: Some(Value::List {
                    vals: stream_test_1,
                    span: Span::test_data(),
                }),
            },
            Example {
                example: r#"[1 2 stop 3 4] | each while { |it| if $it == 'stop' { null } else { $"Output: ($it)" } }"#,
                description: "Output elements till reaching 'stop'",
                result: Some(Value::List {
                    vals: stream_test_2,
                    span: Span::test_data(),
                }),
            },
            Example {
                example: r#"[1 2 3] | each while -n { |it| if $it.item < 2 { $"value ($it.item) at ($it.index)!"} else { null } }"#,
                description: "Iterate over each element, print the matching value and its index",
                result: Some(Value::List {
                    vals: vec![Value::String {
                        val: "value 1 at 0!".to_string(),
                        span: Span::test_data(),
                    }],
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
            PipelineData::Value(Value::Range { .. }, ..)
            | PipelineData::Value(Value::List { .. }, ..)
            | PipelineData::ListStream { .. } => Ok(input
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

                    match eval_block(
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
            PipelineData::ExternalStream { stdout: None, .. } => Ok(PipelineData::new(call.head)),
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

                    match eval_block(
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
            PipelineData::Value(x, ..) => {
                if let Some(var) = block.signature.get_positional(0) {
                    if let Some(var_id) = &var.var_id {
                        stack.add_var(*var_id, x.clone());
                    }
                }

                eval_block(
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

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(EachWhile {})
    }
}
