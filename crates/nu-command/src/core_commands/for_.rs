use nu_engine::{eval_block, eval_expression, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Block, Command, EngineState, Stack};
use nu_protocol::{Category, Example, ListStream, PipelineData, Signature, SyntaxShape, Value};

#[derive(Clone)]
pub struct For;

impl Command for For {
    fn name(&self) -> &str {
        "for"
    }

    fn usage(&self) -> &str {
        "Loop over a range"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("for")
            .required(
                "var_name",
                SyntaxShape::VarWithOptType,
                "name of the looping variable",
            )
            .required(
                "range",
                SyntaxShape::Keyword(b"in".to_vec(), Box::new(SyntaxShape::Any)),
                "range of the loop",
            )
            .required("block", SyntaxShape::Block, "the block to run")
            .switch(
                "numbered",
                "returned a numbered item ($it.index and $it.item)",
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
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
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
        let values = eval_expression(engine_state, stack, keyword_expr)?;

        let block: Block = call.req(engine_state, stack, 2)?;

        let numbered = call.has_flag("numbered");

        let ctrlc = engine_state.ctrlc.clone();
        let engine_state = engine_state.clone();
        let block = engine_state.get_block(block.block_id).clone();
        let redirect_stdout = call.redirect_stdout;
        let redirect_stderr = call.redirect_stderr;

        match values {
            Value::List { vals, .. } => {
                for (idx, x) in ListStream::from_stream(vals.into_iter(), ctrlc).enumerate() {
                    // with_env() is used here to ensure that each iteration uses
                    // a different set of environment variables.
                    // Hence, a 'cd' in the first loop won't affect the next loop.

                    stack.add_var(
                        var_id,
                        if numbered {
                            Value::Record {
                                cols: vec!["index".into(), "item".into()],
                                vals: vec![
                                    Value::Int {
                                        val: idx as i64,
                                        span: head,
                                    },
                                    x,
                                ],
                                span: head,
                            }
                        } else {
                            x
                        },
                    );

                    //let block = engine_state.get_block(block_id);
                    eval_block(
                        &engine_state,
                        stack,
                        &block,
                        PipelineData::new(head),
                        redirect_stdout,
                        redirect_stderr,
                    )?
                    .into_value(head);
                }
            }
            Value::Range { val, .. } => {
                for (idx, x) in val.into_range_iter(ctrlc)?.enumerate() {
                    stack.add_var(
                        var_id,
                        if numbered {
                            Value::Record {
                                cols: vec!["index".into(), "item".into()],
                                vals: vec![
                                    Value::Int {
                                        val: idx as i64,
                                        span: head,
                                    },
                                    x,
                                ],
                                span: head,
                            }
                        } else {
                            x
                        },
                    );

                    //let block = engine_state.get_block(block_id);
                    eval_block(
                        &engine_state,
                        stack,
                        &block,
                        PipelineData::new(head),
                        redirect_stdout,
                        redirect_stderr,
                    )?
                    .into_value(head);
                }
            }
            x => {
                stack.add_var(var_id, x);

                eval_block(
                    &engine_state,
                    stack,
                    &block,
                    PipelineData::new(head),
                    redirect_stdout,
                    redirect_stderr,
                )?
                .into_value(head);
            }
        }
        Ok(PipelineData::new(head))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Echo the square of each integer",
                example: "for x in [1 2 3] { print ($x * $x) }",
                result: None,
            },
            Example {
                description: "Work with elements of a range",
                example: "for $x in 1..3 { print $x }",
                result: None,
            },
            Example {
                description: "Number each item and echo a message",
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
