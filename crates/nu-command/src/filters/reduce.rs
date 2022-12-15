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
                SyntaxShape::Closure(Some(vec![
                    SyntaxShape::Any,
                    SyntaxShape::Any,
                    SyntaxShape::Int,
                ])),
                "reducing function",
            )
            .switch(
                "numbered",
                "iterate with an index (deprecated; use a 3-parameter closure instead)",
                Some('n'),
            )
    }

    fn usage(&self) -> &str {
        "Aggregate a list to a single value using an accumulator closure."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["map", "fold", "foldl"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[ 1 2 3 4 ] | reduce {|it, acc| $it + $acc }",
                description: "Sum values of a list (same as 'math sum')",
                result: Some(Value::int(10, Span::test_data())),
            },
            Example {
                example: "[ 8 7 6 ] | reduce {|it, acc, ind| $acc + $it + $ind }",
                description: "Sum values of a list, plus their indexes",
                result: Some(Value::int(22, Span::test_data())),
            },
            Example {
                example: "[ 1 2 3 4 ] | reduce -f 10 {|it, acc| $acc + $it }",
                description: "Sum values with a starting value (fold)",
                result: Some(Value::int(20, Span::test_data())),
            },
            Example {
                example: r#"[ i o t ] | reduce -f "Arthur, King of the Britons" {|it, acc| $acc | str replace -a $it "X" }"#,
                description: "Replace selected characters in a string with 'X'",
                result: Some(Value::test_string("ArXhur, KXng Xf Xhe BrXXXns")),
            },
            Example {
                example: r#"['foo.gz', 'bar.gz', 'baz.gz'] | reduce -f '' {|str all ind| $"($all)(if $ind != 0 {'; '})($ind + 1)-($str)" }"#,
                description:
                    "Add ascending numbers to each of the filenames, and join with semicolons.",
                result: Some(Value::test_string("1-foo.gz; 2-bar.gz; 3-baz.gz")),
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

        // To enumerate over the input (for the index argument),
        // it must be converted into an iterator using into_iter().
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

            // Element argument
            if let Some(var) = block.signature.get_positional(0) {
                if let Some(var_id) = &var.var_id {
                    stack.add_var(*var_id, x);
                }
            }

            // Accumulator argument
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
            // Optional third index argument
            if let Some(var) = block.signature.get_positional(2) {
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

            acc = eval_block(
                engine_state,
                &mut stack,
                block,
                PipelineData::empty(),
                // redirect stdout until its the last input value
                redirect_stdout || input_iter.peek().is_some(),
                redirect_stderr,
            )?
            .into_value(span);

            if nu_utils::ctrl_c::was_pressed(&ctrlc) {
                break;
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
