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
                "then_block",
                SyntaxShape::Block(Some(vec![])),
                "block to run if check succeeds",
            )
            .optional(
                "else_expression",
                SyntaxShape::Keyword(b"else".to_vec(), Box::new(SyntaxShape::Expression)),
                "expression or block to run if check fails",
            )
            .category(Category::Core)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        caller_stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let cond = &call.positional[0];
        let then_block: CaptureBlock = call.req(engine_state, caller_stack, 1)?;
        let else_case = call.positional.get(2);

        let result = eval_expression(engine_state, caller_stack, cond)?;
        match &result {
            Value::Bool { val, .. } => {
                if *val {
                    let block = engine_state.get_block(then_block.block_id);
                    let mut callee_stack = caller_stack.captures_to_stack(&then_block.captures);
                    let result = eval_block(
                        engine_state,
                        &mut callee_stack,
                        block,
                        input,
                        call.redirect_stdout,
                        call.redirect_stderr,
                    );
                    let caller_env_vars = caller_stack.get_env_var_names(engine_state);

                    // remove env vars that are present in the caller but not in the callee
                    // (the callee hid them)
                    for var in caller_env_vars.iter() {
                        if !callee_stack.has_env_var(engine_state, var) {
                            caller_stack.remove_env_var(engine_state, var);
                        }
                    }

                    // add new env vars from callee to caller
                    for env_vars in callee_stack.env_vars {
                        for (var, value) in env_vars {
                            caller_stack.add_env_var(var, value);
                        }
                    }

                    result
                } else if let Some(else_case) = else_case {
                    if let Some(else_expr) = else_case.as_keyword() {
                        if let Some(block_id) = else_expr.as_block() {
                            let result = eval_expression(engine_state, caller_stack, else_expr)?;
                            let else_block: CaptureBlock = FromValue::from_value(&result)?;

                            let mut callee_stack =
                                caller_stack.captures_to_stack(&else_block.captures);
                            let block = engine_state.get_block(block_id);
                            let result = eval_block(
                                engine_state,
                                &mut callee_stack,
                                block,
                                input,
                                call.redirect_stdout,
                                call.redirect_stderr,
                            );

                            let caller_env_vars = caller_stack.get_env_var_names(engine_state);

                            // remove env vars that are present in the caller but not in the callee
                            // (the callee hid them)
                            for var in caller_env_vars.iter() {
                                if !callee_stack.has_env_var(engine_state, var) {
                                    caller_stack.remove_env_var(engine_state, var);
                                }
                            }

                            // add new env vars from callee to caller
                            for env_vars in callee_stack.env_vars {
                                for (var, value) in env_vars {
                                    caller_stack.add_env_var(var, value);
                                }
                            }

                            result
                        } else {
                            eval_expression(engine_state, caller_stack, else_expr)
                                .map(|x| x.into_pipeline_data())
                        }
                    } else {
                        eval_expression(engine_state, caller_stack, else_case)
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
