use nu_engine::{eval_block, eval_expression, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{CaptureBlock, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, FromValue, IntoPipelineData, PipelineData, ShellError, Signature,
    SyntaxShape, Value,
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
            .required("cond", SyntaxShape::Expression, "condition to check")
            .required(
                "then-block",
                SyntaxShape::Block(Some(vec![])),
                "block to run if check succeeds",
            )
            .optional(
                "else-expression",
                SyntaxShape::Keyword(b"else".to_vec(), Box::new(SyntaxShape::Expression)),
                "expression or block to run if check fails",
            )
            .category(Category::Core)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let cond = &call.positional[0];
        let then_block: CaptureBlock = call.req(engine_state, stack, 1)?;
        let else_case = call.positional.get(2);

        let result = eval_expression(engine_state, stack, cond)?;
        match &result {
            Value::Bool { val, .. } => {
                if *val {
                    let block = engine_state.get_block(then_block.block_id);
                    let mut stack = stack.captures_to_stack(&then_block.captures);
                    eval_block(engine_state, &mut stack, block, input)
                } else if let Some(else_case) = else_case {
                    if let Some(else_expr) = else_case.as_keyword() {
                        if let Some(block_id) = else_expr.as_block() {
                            let result = eval_expression(engine_state, stack, else_expr)?;
                            let else_block: CaptureBlock = FromValue::from_value(&result)?;

                            let mut stack = stack.captures_to_stack(&else_block.captures);
                            let block = engine_state.get_block(block_id);
                            eval_block(engine_state, &mut stack, block, input)
                        } else {
                            eval_expression(engine_state, stack, else_expr)
                                .map(|x| x.into_pipeline_data())
                        }
                    } else {
                        eval_expression(engine_state, stack, else_case)
                            .map(|x| x.into_pipeline_data())
                    }
                } else {
                    Ok(PipelineData::new(call.head))
                }
            }
            x => Err(ShellError::CantConvert(
                "bool".into(),
                x.get_type().to_string(),
                result.span()?,
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
