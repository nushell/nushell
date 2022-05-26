use nu_engine::{eval_block, eval_expression, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{CaptureBlock, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, ListStream, PipelineData, Signature, Span,
    SyntaxShape, Value,
};

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
            .required(
                "block",
                SyntaxShape::Block(Some(vec![])),
                "the block to run",
            )
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
  https://www.nushell.sh/book/thinking_in_nushell.html"#
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

        let capture_block: CaptureBlock = call.req(engine_state, stack, 2)?;

        let numbered = call.has_flag("numbered");

        let ctrlc = engine_state.ctrlc.clone();
        let engine_state = engine_state.clone();
        let block = engine_state.get_block(capture_block.block_id).clone();
        let mut stack = stack.captures_to_stack(&capture_block.captures);
        let orig_env_vars = stack.env_vars.clone();
        let orig_env_hidden = stack.env_hidden.clone();
        let redirect_stdout = call.redirect_stdout;
        let redirect_stderr = call.redirect_stderr;

        match values {
            Value::List { vals, .. } => {
                Ok(ListStream::from_stream(vals.into_iter(), ctrlc.clone())
                    .enumerate()
                    .map(move |(idx, x)| {
                        stack.with_env(&orig_env_vars, &orig_env_hidden);

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
                        match eval_block(
                            &engine_state,
                            &mut stack,
                            &block,
                            PipelineData::new(head),
                            redirect_stdout,
                            redirect_stderr,
                        ) {
                            Ok(pipeline_data) => pipeline_data.into_value(head),
                            Err(error) => Value::Error { error },
                        }
                    })
                    .filter(|x| !x.is_nothing())
                    .into_pipeline_data(ctrlc))
            }
            Value::Range { val, .. } => Ok(val
                .into_range_iter(ctrlc.clone())?
                .enumerate()
                .map(move |(idx, x)| {
                    stack.with_env(&orig_env_vars, &orig_env_hidden);

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
                    match eval_block(
                        &engine_state,
                        &mut stack,
                        &block,
                        PipelineData::new(head),
                        redirect_stdout,
                        redirect_stderr,
                    ) {
                        Ok(pipeline_data) => pipeline_data.into_value(head),
                        Err(error) => Value::Error { error },
                    }
                })
                .filter(|x| !x.is_nothing())
                .into_pipeline_data(ctrlc)),
            x => {
                stack.add_var(var_id, x);

                eval_block(
                    &engine_state,
                    &mut stack,
                    &block,
                    PipelineData::new(head),
                    redirect_stdout,
                    redirect_stderr,
                )
            }
        }
    }

    fn examples(&self) -> Vec<Example> {
        let span = Span::test_data();
        vec![
            Example {
                description: "Echo the square of each integer",
                example: "for x in [1 2 3] { $x * $x }",
                result: Some(Value::List {
                    vals: vec![
                        Value::Int { val: 1, span },
                        Value::Int { val: 4, span },
                        Value::Int { val: 9, span },
                    ],
                    span,
                }),
            },
            Example {
                description: "Work with elements of a range",
                example: "for $x in 1..3 { $x }",
                result: Some(Value::List {
                    vals: vec![
                        Value::Int { val: 1, span },
                        Value::Int { val: 2, span },
                        Value::Int { val: 3, span },
                    ],
                    span,
                }),
            },
            Example {
                description: "Number each item and echo a message",
                example: "for $it in ['bob' 'fred'] --numbered { $\"($it.index) is ($it.item)\" }",
                result: Some(Value::List {
                    vals: vec![
                        Value::String {
                            val: "0 is bob".into(),
                            span,
                        },
                        Value::String {
                            val: "1 is fred".into(),
                            span,
                        },
                    ],
                    span,
                }),
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
