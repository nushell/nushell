use super::utils::chain_error_with_input;
use nu_engine::{eval_block_with_early_return, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, ShellError,
    Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Each;

impl Command for Each {
    fn name(&self) -> &str {
        "each"
    }

    fn usage(&self) -> &str {
        "Run a closure on each row of the input list, creating a new list with the results."
    }

    fn extra_usage(&self) -> &str {
        r#"Since tables are lists of records, passing a table into 'each' will
iterate over each record, not necessarily each cell within it.

Avoid passing single records to this command. Since a record is a
one-row structure, 'each' will only run once, behaving similar to 'do'.
To iterate over a record's values, try converting it to a table
with 'transpose' first."#
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["for", "loop", "iterate", "map"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("each")
            .input_output_types(vec![(
                Type::List(Box::new(Type::Any)),
                Type::List(Box::new(Type::Any)),
            )])
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any, SyntaxShape::Int])),
                "the closure to run",
            )
            .switch("keep-empty", "keep empty result cells", Some('k'))
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
            Value::int(6, Span::test_data()),
        ];

        let stream_test_2 = vec![
            Value::Nothing {
                span: Span::test_data(),
            },
            Value::string("found 2!", Span::test_data()),
            Value::Nothing {
                span: Span::test_data(),
            },
        ];

        vec![
            Example {
                example: "[1 2 3] | each {|e| 2 * $e }",
                description: "Multiplies elements in list",
                result: Some(Value::List {
                    vals: stream_test_1,
                    span: Span::test_data(),
                }),
            },
            Example {
                example: r#"[1 2 3 2] | each {|e| if $e == 2 { "two" } }"#,
                description: "Produce a list that has \"two\" for each 2 in the input",
                result: Some(Value::List {
                    vals: vec![
                        Value::string("two", Span::test_data()),
                        Value::string("two", Span::test_data()),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: r#"[1 2 3] | each {|el ind| if $el == 2 { $"found 2 at ($ind)!"} }"#,
                description:
                    "Iterate over each element, producing a list showing indexes of any 2s",
                result: Some(Value::List {
                    vals: vec![Value::string("found 2 at 1!", Span::test_data())],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: r#"[1 2 3] | each --keep-empty {|e| if $e == 2 { "found 2!"} }"#,
                description: "Iterate over each element, keeping all results",
                result: Some(Value::List {
                    vals: stream_test_2,
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
        let keep_empty = call.has_flag("keep-empty");

        let metadata = input.metadata();
        let ctrlc = engine_state.ctrlc.clone();
        let outer_ctrlc = engine_state.ctrlc.clone();
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
                .map_while(move |(idx, x)| {
                    // with_env() is used here to ensure that each iteration uses
                    // a different set of environment variables.
                    // Hence, a 'cd' in the first loop won't affect the next loop.
                    stack.with_env(&orig_env_vars, &orig_env_hidden);

                    if let Some(var) = block.signature.get_positional(0) {
                        if let Some(var_id) = &var.var_id {
                            // -n changes the first argument into an {index, item} record.
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

                    let input_span = x.span();
                    match eval_block_with_early_return(
                        &engine_state,
                        &mut stack,
                        &block,
                        x.into_pipeline_data(),
                        redirect_stdout,
                        redirect_stderr,
                    ) {
                        Ok(v) => Some(v.into_value(span)),
                        Err(ShellError::Break(_)) => None,
                        Err(error) => {
                            let error = chain_error_with_input(error, input_span);
                            Some(Value::Error { error })
                        }
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
                .map_while(move |(idx, x)| {
                    // with_env() is used here to ensure that each iteration uses
                    // a different set of environment variables.
                    // Hence, a 'cd' in the first loop won't affect the next loop.
                    stack.with_env(&orig_env_vars, &orig_env_hidden);

                    let x = match x {
                        Ok(x) => x,
                        Err(ShellError::Break(_)) => return None,
                        Err(err) => return Some(Value::Error { error: err }),
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

                    let input_span = x.span();
                    match eval_block_with_early_return(
                        &engine_state,
                        &mut stack,
                        &block,
                        x.into_pipeline_data(),
                        redirect_stdout,
                        redirect_stderr,
                    ) {
                        Ok(v) => Some(v.into_value(span)),
                        Err(ShellError::Break(_)) => None,
                        Err(error) => {
                            let error = chain_error_with_input(error, input_span);
                            Some(Value::Error { error })
                        }
                    }
                })
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
        .and_then(|x| {
            x.filter(
                move |x| if !keep_empty { !x.is_nothing() } else { true },
                outer_ctrlc,
            )
        })
        .map(|x| x.set_metadata(metadata))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Each {})
    }
}
