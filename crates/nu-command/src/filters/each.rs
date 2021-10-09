use nu_engine::eval_block;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{Example, IntoValueStream, Signature, Span, SyntaxShape, Value};

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
            result: Some(Value::Stream {
                stream: stream_test_1.into_iter().into_value_stream(),
                span: Span::unknown(),
            }),
        }]
    }

    fn run(
        &self,
        context: &EvaluationContext,
        call: &Call,
        input: Value,
    ) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
        let block_id = call.positional[0]
            .as_block()
            .expect("internal error: expected block");

        let numbered = call.has_flag("numbered");
        let context = context.clone();
        let span = call.head;

        match input {
            Value::Range { val, .. } => Ok(Value::Stream {
                stream: val
                    .into_iter()
                    .enumerate()
                    .map(move |(idx, x)| {
                        let engine_state = context.engine_state.borrow();
                        let block = engine_state.get_block(block_id);

                        let state = context.enter_scope();

                        if let Some(var) = block.signature.get_positional(0) {
                            if let Some(var_id) = &var.var_id {
                                if numbered {
                                    state.add_var(
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
                                    state.add_var(*var_id, x);
                                }
                            }
                        }

                        match eval_block(&state, block, Value::nothing()) {
                            Ok(v) => v,
                            Err(error) => Value::Error { error },
                        }
                    })
                    .into_value_stream(),
                span: call.head,
            }),
            Value::List { vals: val, .. } => Ok(Value::Stream {
                stream: val
                    .into_iter()
                    .enumerate()
                    .map(move |(idx, x)| {
                        let engine_state = context.engine_state.borrow();
                        let block = engine_state.get_block(block_id);

                        let state = context.enter_scope();
                        if let Some(var) = block.signature.get_positional(0) {
                            if let Some(var_id) = &var.var_id {
                                if numbered {
                                    state.add_var(
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
                                    state.add_var(*var_id, x);
                                }
                            }
                        }

                        match eval_block(&state, block, Value::nothing()) {
                            Ok(v) => v,
                            Err(error) => Value::Error { error },
                        }
                    })
                    .into_value_stream(),
                span: call.head,
            }),
            Value::Stream { stream, .. } => Ok(Value::Stream {
                stream: stream
                    .enumerate()
                    .map(move |(idx, x)| {
                        let engine_state = context.engine_state.borrow();
                        let block = engine_state.get_block(block_id);

                        let state = context.enter_scope();
                        if let Some(var) = block.signature.get_positional(0) {
                            if let Some(var_id) = &var.var_id {
                                if numbered {
                                    state.add_var(
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
                                    state.add_var(*var_id, x);
                                }
                            }
                        }

                        match eval_block(&state, block, Value::nothing()) {
                            Ok(v) => v,
                            Err(error) => Value::Error { error },
                        }
                    })
                    .into_value_stream(),
                span: call.head,
            }),
            Value::Record { cols, vals, .. } => {
                let mut output_cols = vec![];
                let mut output_vals = vec![];

                for (col, val) in cols.into_iter().zip(vals.into_iter()) {
                    let engine_state = context.engine_state.borrow();
                    let block = engine_state.get_block(block_id);

                    let state = context.enter_scope();
                    if let Some(var) = block.signature.get_positional(0) {
                        if let Some(var_id) = &var.var_id {
                            state.add_var(
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

                    match eval_block(&state, block, Value::nothing())? {
                        Value::Record {
                            mut cols, mut vals, ..
                        } => {
                            // TODO check that the lengths match
                            output_cols.append(&mut cols);
                            output_vals.append(&mut vals);
                        }
                        x => {
                            output_cols.push(col);
                            output_vals.push(x);
                        }
                    }
                }

                Ok(Value::Record {
                    cols: output_cols,
                    vals: output_vals,
                    span: call.head,
                })
            }
            x => {
                //TODO: we need to watch to make sure this is okay
                let engine_state = context.engine_state.borrow();
                let block = engine_state.get_block(block_id);

                let state = context.enter_scope();
                if let Some(var) = block.signature.get_positional(0) {
                    if let Some(var_id) = &var.var_id {
                        state.add_var(*var_id, x);
                    }
                }

                eval_block(&state, block, Value::nothing())
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
