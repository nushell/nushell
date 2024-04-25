use nu_engine::{command_prelude::*, get_eval_block, get_eval_expression};
use nu_protocol::ListStream;

#[derive(Clone)]
pub struct For;

impl Command for For {
    fn name(&self) -> &str {
        "for"
    }

    fn usage(&self) -> &str {
        "Loop over a range."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("for")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .allow_variants_without_examples(true)
            .required(
                "var_name",
                SyntaxShape::VarWithOptType,
                "Name of the looping variable.",
            )
            .required(
                "range",
                SyntaxShape::Keyword(b"in".to_vec(), Box::new(SyntaxShape::Any)),
                "Range of the loop.",
            )
            .required("block", SyntaxShape::Block, "The block to run.")
            .switch(
                "numbered",
                "return a numbered item ($it.index and $it.item)",
                Some('n'),
            )
            .creates_scope()
            .category(Category::Core)
    }

    fn extra_usage(&self) -> &str {
        r#"This command is a parser keyword. For details, check:
  https://www.nushell.sh/book/thinking_in_nu.html"#
    }

    fn is_parser_keyword(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let var_id = call
            .positional_nth(0)
            .expect("checked through parser")
            .as_var()
            .expect("internal error: missing variable");

        let keyword_expr = call
            .positional_nth(1)
            .expect("checked through parser")
            .as_keyword()
            .expect("internal error: missing keyword");

        let block_id = call
            .positional_nth(2)
            .expect("checked through parser")
            .as_block()
            .expect("internal error: missing block");

        let eval_expression = get_eval_expression(engine_state);
        let eval_block = get_eval_block(engine_state);

        let value = eval_expression(engine_state, stack, keyword_expr)?;

        let numbered = call.has_flag(engine_state, stack, "numbered")?;

        let ctrlc = engine_state.ctrlc.clone();
        let engine_state = engine_state.clone();
        let block = engine_state.get_block(block_id);

        let stack = &mut stack.push_redirection(None, None);

        let span = value.span();
        match value {
            Value::List { vals, .. } => {
                for (idx, x) in ListStream::from_stream(vals.into_iter(), ctrlc).enumerate() {
                    // with_env() is used here to ensure that each iteration uses
                    // a different set of environment variables.
                    // Hence, a 'cd' in the first loop won't affect the next loop.

                    stack.add_var(
                        var_id,
                        if numbered {
                            Value::record(
                                record! {
                                    "index" => Value::int(idx as i64, head),
                                    "item" => x,
                                },
                                head,
                            )
                        } else {
                            x
                        },
                    );

                    match eval_block(&engine_state, stack, block, PipelineData::empty()) {
                        Err(ShellError::Break { .. }) => {
                            break;
                        }
                        Err(ShellError::Continue { .. }) => {
                            continue;
                        }
                        Err(err) => {
                            return Err(err);
                        }
                        Ok(pipeline) => {
                            let exit_code = pipeline.drain_with_exit_code()?;
                            if exit_code != 0 {
                                return Ok(PipelineData::new_external_stream_with_only_exit_code(
                                    exit_code,
                                ));
                            }
                        }
                    }
                }
            }
            Value::Range { val, .. } => {
                for (idx, x) in val.into_range_iter(span, ctrlc).enumerate() {
                    stack.add_var(
                        var_id,
                        if numbered {
                            Value::record(
                                record! {
                                    "index" => Value::int(idx as i64, head),
                                    "item" => x,
                                },
                                head,
                            )
                        } else {
                            x
                        },
                    );

                    match eval_block(&engine_state, stack, block, PipelineData::empty()) {
                        Err(ShellError::Break { .. }) => {
                            break;
                        }
                        Err(ShellError::Continue { .. }) => {
                            continue;
                        }
                        Err(err) => {
                            return Err(err);
                        }
                        Ok(pipeline) => {
                            let exit_code = pipeline.drain_with_exit_code()?;
                            if exit_code != 0 {
                                return Ok(PipelineData::new_external_stream_with_only_exit_code(
                                    exit_code,
                                ));
                            }
                        }
                    }
                }
            }
            x => {
                stack.add_var(var_id, x);

                eval_block(&engine_state, stack, block, PipelineData::empty())?.into_value(head);
            }
        }
        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Print the square of each integer",
                example: "for x in [1 2 3] { print ($x * $x) }",
                result: None,
            },
            Example {
                description: "Work with elements of a range",
                example: "for $x in 1..3 { print $x }",
                result: None,
            },
            Example {
                description: "Number each item and print a message",
                example:
                    "for $it in ['bob' 'fred'] --numbered { print $\"($it.index) is ($it.item)\" }",
                result: None,
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(For {})
    }
}
