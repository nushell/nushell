use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, Signature,
    Span, SyntaxShape, Type, Value,
};
use rayon::prelude::*;

use super::utils::chain_error_with_input;

#[derive(Clone)]
pub struct ParEach;

impl Command for ParEach {
    fn name(&self) -> &str {
        "par-each"
    }

    fn usage(&self) -> &str {
        "Run a closure on each element of input in parallel"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("par-each")
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
        vec![
            Example {
                example: "[1 2 3] | par-each { |it| 2 * $it }",
                description: "Multiplies elements in list",
                result: None,
            },
            Example {
                example: r#"[1 2 3] | par-each -n { |it| if $it.item == 2 { echo $"found 2 at ($it.index)!"} }"#,
                description: "Iterate over each element, print the matching value and its index",
                result: Some(Value::List {
                    vals: vec![Value::String {
                        val: "found 2 at 1!".to_string(),
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
        let block_id = capture_block.block_id;
        let mut stack = stack.captures_to_stack(&capture_block.captures);
        let span = call.head;
        let redirect_stdout = call.redirect_stdout;
        let redirect_stderr = call.redirect_stderr;

        match input {
            PipelineData::Value(Value::Range { val, .. }, ..) => Ok(val
                .into_range_iter(ctrlc.clone())?
                .enumerate()
                .par_bridge()
                .map(move |(idx, x)| {
                    let block = engine_state.get_block(block_id);

                    let mut stack = stack.clone();

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

                    let val_span = x.span();
                    match eval_block(
                        engine_state,
                        &mut stack,
                        block,
                        x.into_pipeline_data(),
                        redirect_stdout,
                        redirect_stderr,
                    ) {
                        Ok(v) => v,
                        Err(error) => Value::Error {
                            error: chain_error_with_input(error, val_span),
                        }
                        .into_pipeline_data(),
                    }
                })
                .collect::<Vec<_>>()
                .into_iter()
                .flatten()
                .into_pipeline_data(ctrlc)),
            PipelineData::Value(Value::List { vals: val, .. }, ..) => Ok(val
                .into_iter()
                .enumerate()
                .par_bridge()
                .map(move |(idx, x)| {
                    let block = engine_state.get_block(block_id);

                    let mut stack = stack.clone();

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

                    let val_span = x.span();
                    match eval_block(
                        engine_state,
                        &mut stack,
                        block,
                        x.into_pipeline_data(),
                        redirect_stdout,
                        redirect_stderr,
                    ) {
                        Ok(v) => v,
                        Err(error) => Value::Error {
                            error: chain_error_with_input(error, val_span),
                        }
                        .into_pipeline_data(),
                    }
                })
                .collect::<Vec<_>>()
                .into_iter()
                .flatten()
                .into_pipeline_data(ctrlc)),
            PipelineData::ListStream(stream, ..) => Ok(stream
                .enumerate()
                .par_bridge()
                .map(move |(idx, x)| {
                    let block = engine_state.get_block(block_id);

                    let mut stack = stack.clone();

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

                    let val_span = x.span();
                    match eval_block(
                        engine_state,
                        &mut stack,
                        block,
                        x.into_pipeline_data(),
                        redirect_stdout,
                        redirect_stderr,
                    ) {
                        Ok(v) => v,
                        Err(error) => Value::Error {
                            error: chain_error_with_input(error, val_span),
                        }
                        .into_pipeline_data(),
                    }
                })
                .collect::<Vec<_>>()
                .into_iter()
                .flatten()
                .into_pipeline_data(ctrlc)),
            PipelineData::ExternalStream { stdout: None, .. } => Ok(PipelineData::new(call.head)),
            PipelineData::ExternalStream {
                stdout: Some(stream),
                ..
            } => Ok(stream
                .enumerate()
                .par_bridge()
                .map(move |(idx, x)| {
                    let x = match x {
                        Ok(x) => x,
                        Err(err) => return Value::Error { error: err }.into_pipeline_data(),
                    };

                    let block = engine_state.get_block(block_id);

                    let mut stack = stack.clone();

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
                        engine_state,
                        &mut stack,
                        block,
                        x.into_pipeline_data(),
                        redirect_stdout,
                        redirect_stderr,
                    ) {
                        Ok(v) => v,
                        Err(error) => Value::Error { error }.into_pipeline_data(),
                    }
                })
                .collect::<Vec<_>>()
                .into_iter()
                .flatten()
                .into_pipeline_data(ctrlc)),
            PipelineData::Value(x, ..) => {
                let block = engine_state.get_block(block_id);

                if let Some(var) = block.signature.get_positional(0) {
                    if let Some(var_id) = &var.var_id {
                        stack.add_var(*var_id, x.clone());
                    }
                }

                eval_block(
                    engine_state,
                    &mut stack,
                    block,
                    x.into_pipeline_data(),
                    redirect_stdout,
                    redirect_stderr,
                )
            }
        }
        .map(|res| res.set_metadata(metadata))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(ParEach {})
    }
}
