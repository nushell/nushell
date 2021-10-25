use nu_engine::eval_block;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Example, IntoPipelineData, PipelineData, Signature, Span, SyntaxShape, Value};

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
    }

    fn examples(&self) -> Vec<Example> {
        let stream_test_1 = vec![
            Value::Int {
                val: 2,
                span: Span::unknown(),
            },
            Value::Int {
                val: 4,
                span: Span::unknown(),
            },
            Value::Int {
                val: 6,
                span: Span::unknown(),
            },
        ];

        vec![Example {
            example: "[1 2 3] | each { 2 * $it }",
            description: "Multiplies elements in list",
            result: Some(Value::List {
                vals: stream_test_1,
                span: Span::unknown(),
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
        let block_id = call.positional[0]
            .as_block()
            .expect("internal error: expected block");

        let numbered = call.has_flag("numbered");
        let engine_state = engine_state.clone();
        let stack = stack.clone();
        let span = call.head;

        match input {
            PipelineData::Value(Value::Range { val, .. }) => Ok(val
                .into_range_iter()?
                .enumerate()
                .map(move |(idx, x)| {
                    let block = engine_state.get_block(block_id);

                    let mut stack = stack.enter_scope();

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

                    match eval_block(&engine_state, &mut stack, block, PipelineData::new()) {
                        Ok(v) => v,
                        Err(error) => Value::Error { error }.into_pipeline_data(),
                    }
                })
                .flatten()
                .into_pipeline_data()),
            PipelineData::Value(Value::List { vals: val, .. }) => Ok(val
                .into_iter()
                .enumerate()
                .map(move |(idx, x)| {
                    let block = engine_state.get_block(block_id);

                    let mut stack = stack.enter_scope();
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

                    match eval_block(&engine_state, &mut stack, block, PipelineData::new()) {
                        Ok(v) => v,
                        Err(error) => Value::Error { error }.into_pipeline_data(),
                    }
                })
                .flatten()
                .into_pipeline_data()),
            PipelineData::Stream(stream) => Ok(stream
                .enumerate()
                .map(move |(idx, x)| {
                    let block = engine_state.get_block(block_id);

                    let mut stack = stack.enter_scope();
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

                    match eval_block(&engine_state, &mut stack, block, PipelineData::new()) {
                        Ok(v) => v,
                        Err(error) => Value::Error { error }.into_pipeline_data(),
                    }
                })
                .flatten()
                .into_pipeline_data()),
            PipelineData::Value(Value::Record { cols, vals, .. }) => {
                let mut output_cols = vec![];
                let mut output_vals = vec![];

                for (col, val) in cols.into_iter().zip(vals.into_iter()) {
                    let block = engine_state.get_block(block_id);

                    let mut stack = stack.enter_scope();
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

                    match eval_block(&engine_state, &mut stack, block, PipelineData::new())? {
                        PipelineData::Value(Value::Record {
                            mut cols, mut vals, ..
                        }) => {
                            // TODO check that the lengths match when traversing record
                            output_cols.append(&mut cols);
                            output_vals.append(&mut vals);
                        }
                        x => {
                            output_cols.push(col);
                            output_vals.push(x.into_value());
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
            PipelineData::Value(x) => {
                let block = engine_state.get_block(block_id);

                let mut stack = stack.enter_scope();
                if let Some(var) = block.signature.get_positional(0) {
                    if let Some(var_id) = &var.var_id {
                        stack.add_var(*var_id, x);
                    }
                }

                eval_block(&engine_state, &mut stack, block, PipelineData::new())
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
