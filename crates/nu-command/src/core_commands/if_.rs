use nu_engine::{eval_block, eval_expression, eval_expression_with_input, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Block, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct If;

impl Command for If {
    fn name(&self) -> &str {
        "if"
    }

    fn usage(&self) -> &str {
        "Conditionally run a block."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("if")
            .input_output_types(vec![(Type::Any, Type::Any)])
            .required("cond", SyntaxShape::Expression, "condition to check")
            .required(
                "then_block",
                SyntaxShape::Block,
                "block to run if check succeeds",
            )
            .optional(
                "else_expression",
                SyntaxShape::Keyword(b"else".to_vec(), Box::new(SyntaxShape::Expression)),
                "expression or block to run if check fails",
            )
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
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let cond = call.positional_nth(0).expect("checked through parser");
        let then_block: Block = call.req(engine_state, stack, 1)?;
        let else_case = call.positional_nth(2);

        let result = eval_expression(engine_state, stack, cond)?;
        match &result {
            Value::Bool { val, .. } => {
                if *val {
                    let block = engine_state.get_block(then_block.block_id);
                    eval_block(
                        engine_state,
                        stack,
                        block,
                        input,
                        call.redirect_stdout,
                        call.redirect_stderr,
                    )
                } else if let Some(else_case) = else_case {
                    if let Some(else_expr) = else_case.as_keyword() {
                        if let Some(block_id) = else_expr.as_block() {
                            let block = engine_state.get_block(block_id);
                            eval_block(
                                engine_state,
                                stack,
                                block,
                                input,
                                call.redirect_stdout,
                                call.redirect_stderr,
                            )
                        } else {
                            eval_expression_with_input(
                                engine_state,
                                stack,
                                else_expr,
                                input,
                                call.redirect_stdout,
                                call.redirect_stderr,
                            )
                            .map(|res| res.0)
                        }
                    } else {
                        eval_expression_with_input(
                            engine_state,
                            stack,
                            else_case,
                            input,
                            call.redirect_stdout,
                            call.redirect_stderr,
                        )
                        .map(|res| res.0)
                    }
                } else {
                    Ok(PipelineData::new(call.head))
                }
            }
            x => Err(ShellError::CantConvert(
                "bool".into(),
                x.get_type().to_string(),
                result.span()?,
                None,
            )),
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Output a value if a condition matches, otherwise return nothing",
                example: "if 2 < 3 { 'yes!' }",
                result: Some(Value::test_string("yes!")),
            },
            Example {
                description: "Output a value if a condition matches, else return another value",
                example: "if 5 < 3 { 'yes!' } else { 'no!' }",
                result: Some(Value::test_string("no!")),
            },
            Example {
                description: "Chain multiple if's together",
                example: "if 5 < 3 { 'yes!' } else if 4 < 5 { 'no!' } else { 'okay!' }",
                result: Some(Value::test_string("no!")),
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

        test_examples(If {})
    }
}
