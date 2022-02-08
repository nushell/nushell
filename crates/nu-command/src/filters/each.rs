use nu_engine::{eval_block_with_redirect, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{CaptureBlock, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, Signature,
    Span, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Each;

impl Command for Each {
    fn name(&self) -> &str {
        "each"
    }

    fn usage(&self) -> &str {
        "Run a block on each element of input"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("each")
            .required(
                "block",
                SyntaxShape::Block(Some(vec![SyntaxShape::Any])),
                "the block to run",
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
            Value::Int {
                val: 6,
                span: Span::test_data(),
            },
        ];

        vec![Example {
            example: "[1 2 3] | each { 2 * $it }",
            description: "Multiplies elements in list",
            result: Some(Value::List {
                vals: stream_test_1,
                span: Span::test_data(),
            }),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let capture_block: CaptureBlock = call.req(engine_state, stack, 0)?;

        let numbered = call.has_flag("numbered");
        let ctrlc = engine_state.ctrlc.clone();
        let engine_state = engine_state.clone();
        let block = engine_state.get_block(capture_block.block_id).clone();
        let mut stack = stack.captures_to_stack(&capture_block.captures);
        let orig_env_vars = stack.env_vars.clone();
        let orig_env_hidden = stack.env_hidden.clone();
        let span = call.head;

        match input {
            PipelineData::Value(Value::Range { .. }, ..)
            | PipelineData::Value(Value::List { .. }, ..)
            | PipelineData::ListStream { .. } => Ok(input
                .into_iter()
                .enumerate()
                .map(move |(idx, x)| {
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
                                            x,
                                        ],
                                        span,
                                    },
                                );
                            } else {
                                stack.add_var(*var_id, x);
                            }
                        }
                    }

                    match eval_block_with_redirect(
                        &engine_state,
                        &mut stack,
                        &block,
                        PipelineData::new(span),
                    ) {
                        Ok(v) => v.into_value(span),
                        Err(error) => Value::Error { error },
                    }
                })
                .into_pipeline_data(ctrlc)),
            PipelineData::RawStream(stream, ..) => Ok(stream
                .into_iter()
                .enumerate()
                .map(move |(idx, x)| {
                    stack.with_env(&orig_env_vars, &orig_env_hidden);

                    let x = match x {
                        Ok(x) => x,
                        Err(err) => return Value::Error { error: err },
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
                                            x,
                                        ],
                                        span,
                                    },
                                );
                            } else {
                                stack.add_var(*var_id, x);
                            }
                        }
                    }

                    match eval_block_with_redirect(
                        &engine_state,
                        &mut stack,
                        &block,
                        PipelineData::new(span),
                    ) {
                        Ok(v) => v.into_value(span),
                        Err(error) => Value::Error { error },
                    }
                })
                .into_pipeline_data(ctrlc)),
            PipelineData::Value(Value::Record { cols, vals, .. }, ..) => {
                let mut output_cols = vec![];
                let mut output_vals = vec![];

                for (col, val) in cols.into_iter().zip(vals.into_iter()) {
                    //let block = engine_state.get_block(block_id);

                    stack.with_env(&orig_env_vars, &orig_env_hidden);

                    if let Some(var) = block.signature.get_positional(0) {
                        if let Some(var_id) = &var.var_id {
                            stack.add_var(
                                *var_id,
                                Value::Record {
                                    cols: vec!["column".into(), "value".into()],
                                    vals: vec![
                                        Value::String {
                                            val: col.clone(),
                                            span: call.head,
                                        },
                                        val,
                                    ],
                                    span: call.head,
                                },
                            );
                        }
                    }

                    match eval_block_with_redirect(
                        &engine_state,
                        &mut stack,
                        &block,
                        PipelineData::new(span),
                    )? {
                        PipelineData::Value(
                            Value::Record {
                                mut cols, mut vals, ..
                            },
                            ..,
                        ) => {
                            // TODO check that the lengths match when traversing record
                            output_cols.append(&mut cols);
                            output_vals.append(&mut vals);
                        }
                        x => {
                            output_cols.push(col);
                            output_vals.push(x.into_value(span));
                        }
                    }
                }

                Ok(Value::Record {
                    cols: output_cols,
                    vals: output_vals,
                    span: call.head,
                }
                .into_pipeline_data())
            }
            PipelineData::Value(x, ..) => {
                //let block = engine_state.get_block(block_id);

                if let Some(var) = block.signature.get_positional(0) {
                    if let Some(var_id) = &var.var_id {
                        stack.add_var(*var_id, x);
                    }
                }

                eval_block_with_redirect(&engine_state, &mut stack, &block, PipelineData::new(span))
            }
        }
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
