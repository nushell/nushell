use nu_engine::{eval_block_with_redirect, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{CaptureBlock, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, Signature,
    SyntaxShape, Value,
};
use rayon::prelude::*;

#[derive(Clone)]
pub struct ParEach;

impl Command for ParEach {
    fn name(&self) -> &str {
        "par-each"
    }

    fn usage(&self) -> &str {
        "Run a block on each element of input in parallel"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("par-each")
            .required(
                "block",
                SyntaxShape::Block(Some(vec![SyntaxShape::Any])),
                "the block to run",
            )
            .switch("numbered", "iterate with an index", Some('n'))
            .category(Category::Filters)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            example: "[1 2 3] | par-each { |it| 2 * $it }",
            description: "Multiplies elements in list",
            result: None,
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
        let block_id = capture_block.block_id;
        let mut stack = stack.captures_to_stack(&capture_block.captures);
        let span = call.head;

        match input {
            PipelineData::Value(Value::Range { val, .. }, ..) => Ok(val
                .into_range_iter()?
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
                        block,
                        PipelineData::new(span),
                    ) {
                        Ok(v) => v,
                        Err(error) => Value::Error { error }.into_pipeline_data(),
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
                        block,
                        PipelineData::new(span),
                    ) {
                        Ok(v) => v,
                        Err(error) => Value::Error { error }.into_pipeline_data(),
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
                        block,
                        PipelineData::new(span),
                    ) {
                        Ok(v) => v,
                        Err(error) => Value::Error { error }.into_pipeline_data(),
                    }
                })
                .collect::<Vec<_>>()
                .into_iter()
                .flatten()
                .into_pipeline_data(ctrlc)),
            PipelineData::RawStream(stream, ..) => Ok(stream
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
                        block,
                        PipelineData::new(span),
                    ) {
                        Ok(v) => v,
                        Err(error) => Value::Error { error }.into_pipeline_data(),
                    }
                })
                .collect::<Vec<_>>()
                .into_iter()
                .flatten()
                .into_pipeline_data(ctrlc)),
            PipelineData::Value(Value::Record { cols, vals, .. }, ..) => {
                let mut output_cols = vec![];
                let mut output_vals = vec![];

                for (col, val) in cols.into_iter().zip(vals.into_iter()) {
                    let block = engine_state.get_block(block_id);

                    let mut stack = stack.clone();

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
                        block,
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
                let block = engine_state.get_block(block_id);

                if let Some(var) = block.signature.get_positional(0) {
                    if let Some(var_id) = &var.var_id {
                        stack.add_var(*var_id, x);
                    }
                }

                eval_block_with_redirect(&engine_state, &mut stack, block, PipelineData::new(span))
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

        test_examples(ParEach {})
    }
}
