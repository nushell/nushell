use std::sync::atomic::Ordering;

use nu_engine::{eval_block, CallExt};

use nu_protocol::ast::Call;
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Reduce;

impl Command for Reduce {
    fn name(&self) -> &str {
        "reduce"
    }

    fn signature(&self) -> Signature {
        Signature::build("reduce")
            .input_output_types(vec![(Type::List(Box::new(Type::Any)), Type::Any)])
            .named(
                "fold",
                SyntaxShape::Any,
                "reduce with initial value",
                Some('f'),
            )
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any, SyntaxShape::Any])),
                "reducing function",
            )
            .switch("numbered", "iterate with an index", Some('n'))
    }

    fn usage(&self) -> &str {
        "Aggregate a list table to a single value using an accumulator block."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["map", "fold", "foldl"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[ 1 2 3 4 ] | reduce {|it, acc| $it + $acc }",
                description: "Sum values of a list (same as 'math sum')",
                result: Some(Value::Int {
                    val: 10,
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[ 1 2 3 ] | reduce -n {|it, acc| $acc.item + $it.item }",
                description: "Sum values of a list (same as 'math sum')",
                result: Some(Value::Int {
                    val: 6,
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[ 1 2 3 4 ] | reduce -f 10 {|it, acc| $acc + $it }",
                description: "Sum values with a starting value (fold)",
                result: Some(Value::Int {
                    val: 20,
                    span: Span::test_data(),
                }),
            },
            Example {
                example: r#"[ i o t ] | reduce -f "Arthur, King of the Britons" {|it, acc| $acc | str replace -a $it "X" }"#,
                description: "Replace selected characters in a string with 'X'",
                result: Some(Value::String {
                    val: "ArXhur, KXng Xf Xhe BrXXXns".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                example: r#"[ one longest three bar ] | reduce -n { |it, acc|
                    if ($it.item | str length) > ($acc.item | str length) {
                        $it.item
                    } else {
                        $acc.item
                    }
                }"#,
                description: "Find the longest string and its index",
                result: Some(Value::String {
                    val: "longest".to_string(),
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
        let span = call.head;

        let fold: Option<Value> = call.get_flag(engine_state, stack, "fold")?;
        let numbered = call.has_flag("numbered");
        let capture_block: Closure = call.req(engine_state, stack, 0)?;
        let mut stack = stack.captures_to_stack(&capture_block.captures);
        let block = engine_state.get_block(capture_block.block_id);
        let ctrlc = engine_state.ctrlc.clone();

        let orig_env_vars = stack.env_vars.clone();
        let orig_env_hidden = stack.env_hidden.clone();

        let redirect_stdout = call.redirect_stdout;
        let redirect_stderr = call.redirect_stderr;

        let mut input_iter = input.into_iter();

        let (off, start_val) = if let Some(val) = fold {
            (0, val)
        } else if let Some(val) = input_iter.next() {
            (1, val)
        } else {
            return Err(ShellError::GenericError(
                "Expected input".to_string(),
                "needs input".to_string(),
                Some(span),
                None,
                Vec::new(),
            ));
        };

        let mut acc = if numbered {
            Value::Record {
                cols: vec!["index".to_string(), "item".to_string()],
                vals: vec![Value::Int { val: 0, span }, start_val],
                span,
            }
        } else {
            start_val
        };

        let mut input_iter = input_iter
            .enumerate()
            .map(|(idx, x)| {
                if numbered {
                    (
                        idx,
                        Value::Record {
                            cols: vec!["index".to_string(), "item".to_string()],
                            vals: vec![
                                Value::Int {
                                    val: idx as i64 + off,
                                    span,
                                },
                                x,
                            ],
                            span,
                        },
                    )
                } else {
                    (idx, x)
                }
            })
            .peekable();

        while let Some((idx, x)) = input_iter.next() {
            // with_env() is used here to ensure that each iteration uses
            // a different set of environment variables.
            // Hence, a 'cd' in the first loop won't affect the next loop.
            stack.with_env(&orig_env_vars, &orig_env_hidden);

            if let Some(var) = block.signature.get_positional(0) {
                if let Some(var_id) = &var.var_id {
                    stack.add_var(*var_id, x);
                }
            }

            if let Some(var) = block.signature.get_positional(1) {
                if let Some(var_id) = &var.var_id {
                    acc = if numbered {
                        if let Value::Record { .. } = &acc {
                            acc
                        } else {
                            Value::Record {
                                cols: vec!["index".to_string(), "item".to_string()],
                                vals: vec![
                                    Value::Int {
                                        val: idx as i64 + off,
                                        span,
                                    },
                                    acc,
                                ],
                                span,
                            }
                        }
                    } else {
                        acc
                    };

                    stack.add_var(*var_id, acc);
                }
            }

            acc = eval_block(
                engine_state,
                &mut stack,
                block,
                PipelineData::new(span),
                // redirect stdout until its the last input value
                redirect_stdout || input_iter.peek().is_some(),
                redirect_stderr,
            )?
            .into_value(span);

            if let Some(ctrlc) = &ctrlc {
                if ctrlc.load(Ordering::SeqCst) {
                    break;
                }
            }
        }

        Ok(acc.with_span(span).into_pipeline_data())
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
