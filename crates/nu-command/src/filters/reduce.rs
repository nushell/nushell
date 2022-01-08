use nu_engine::{eval_block, CallExt};

use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Reduce;

impl Command for Reduce {
    fn name(&self) -> &str {
        "reduce"
    }

    fn signature(&self) -> Signature {
        Signature::build("reduce")
            .named(
                "fold",
                SyntaxShape::Any,
                "reduce with initial value",
                Some('f'),
            )
            .required(
                "block",
                SyntaxShape::Block(Some(vec![SyntaxShape::Any])),
                "reducing function",
            )
            .switch("numbered", "iterate with an index", Some('n'))
    }

    fn usage(&self) -> &str {
        "Aggregate a list table to a single value using an accumulator block."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[ 1 2 3 4 ] | reduce { $it.acc + $it.item }",
                description: "Sum values of a list (same as 'math sum')",
                result: Some(Value::Int {
                    val: 10,
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[ 1 2 3 4 ] | reduce -f 10 { $it.acc + $it.item }",
                description: "Sum values with a starting value (fold)",
                result: Some(Value::Int {
                    val: 20,
                    span: Span::test_data(),
                }),
            },
            Example {
                example: r#"[ i o t ] | reduce -f "Arthur, King of the Britons" { $it.acc | str find-replace -a $it.item "X" }"#,
                description: "Replace selected characters in a string with 'X'",
                result: Some(Value::String {
                    val: "ArXhur, KXng Xf Xhe BrXXXns".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                example: r#"[ one longest three bar ] | reduce -n {
        if ($it.item | str length) > ($it.acc | str length) {
            $it.item
        } else {
            $it.acc
        }
    }"#,
                description: "Find the longest string and its index",
                result: Some(Value::Record {
                    cols: vec!["index".to_string(), "item".to_string()],
                    vals: vec![
                        Value::Int {
                            val: 3,
                            span: Span::test_data(),
                        },
                        Value::String {
                            val: "longest".to_string(),
                            span: Span::test_data(),
                        },
                    ],
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
    ) -> Result<PipelineData, ShellError> {
        // TODO: How to make this interruptible?
        // TODO: Change the vars to $acc and $it instead of $it.acc and $it.item
        //       (requires parser change)

        let span = call.head;

        let fold: Option<Value> = call.get_flag(engine_state, stack, "fold")?;
        let numbered = call.has_flag("numbered");
        let block = if let Some(block_id) = call.nth(0).and_then(|b| b.as_block()) {
            engine_state.get_block(block_id)
        } else {
            return Err(ShellError::SpannedLabeledError(
                "Internal Error".to_string(),
                "expected block".to_string(),
                span,
            ));
        };

        let mut stack = stack.collect_captures(&block.captures);
        let orig_env_vars = stack.env_vars.clone();
        let orig_env_hidden = stack.env_hidden.clone();

        let mut input_iter = input.into_iter();

        let (off, start_val) = if let Some(val) = fold {
            (0, val)
        } else if let Some(val) = input_iter.next() {
            (1, val)
        } else {
            return Err(ShellError::SpannedLabeledError(
                "Expected input".to_string(),
                "needs input".to_string(),
                span,
            ));
        };

        Ok(input_iter
            .enumerate()
            .fold(start_val, move |acc, (idx, x)| {
                stack.with_env(&orig_env_vars, &orig_env_hidden);

                // if the acc coming from previous iter is indexed, drop the index
                let acc = if let Value::Record { cols, vals, .. } = &acc {
                    if cols.len() == 2 && vals.len() == 2 {
                        if cols[0].eq("index") && cols[1].eq("item") {
                            vals[1].clone()
                        } else {
                            acc
                        }
                    } else {
                        acc
                    }
                } else {
                    acc
                };

                if let Some(var) = block.signature.get_positional(0) {
                    if let Some(var_id) = &var.var_id {
                        let it = if numbered {
                            Value::Record {
                                cols: vec![
                                    "index".to_string(),
                                    "acc".to_string(),
                                    "item".to_string(),
                                ],
                                vals: vec![
                                    Value::Int {
                                        val: idx as i64 + off,
                                        span,
                                    },
                                    acc,
                                    x,
                                ],
                                span,
                            }
                        } else {
                            Value::Record {
                                cols: vec!["acc".to_string(), "item".to_string()],
                                vals: vec![acc, x],
                                span,
                            }
                        };

                        stack.add_var(*var_id, it);
                    }
                }

                let v = match eval_block(engine_state, &mut stack, block, PipelineData::new(span)) {
                    Ok(v) => v.into_value(span),
                    Err(error) => Value::Error { error },
                };

                if numbered {
                    // make sure the output is indexed
                    Value::Record {
                        cols: vec!["index".to_string(), "item".to_string()],
                        vals: vec![
                            Value::Int {
                                val: idx as i64 + off,
                                span,
                            },
                            v,
                        ],
                        span,
                    }
                } else {
                    v
                }
            })
            .into_pipeline_data())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Reduce {})
    }
}
