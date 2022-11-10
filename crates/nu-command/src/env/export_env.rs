use nu_engine::{eval_block, redirect_env, CallExt};
use nu_protocol::{
    ast::Call,
    engine::{Closure, Command, EngineState, Stack},
    Category, Example, PipelineData, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct ExportEnv;

impl Command for ExportEnv {
    fn name(&self) -> &str {
        "export-env"
    }

    fn signature(&self) -> Signature {
        Signature::build("export-env")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required(
                "block",
                SyntaxShape::Block,
                "the block to run to set the environment",
            )
            .category(Category::Env)
    }

    fn usage(&self) -> &str {
        "Run a block and preserve its environment in a current scope."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        caller_stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let capture_block: Closure = call.req(engine_state, caller_stack, 0)?;
        let block = engine_state.get_block(capture_block.block_id);
        let mut callee_stack = caller_stack.captures_to_stack(&capture_block.captures);

        let _ = eval_block(
            engine_state,
            &mut callee_stack,
            block,
            input,
            call.redirect_stdout,
            call.redirect_stderr,
        );

        redirect_env(engine_state, caller_stack, &callee_stack);

        Ok(PipelineData::new(call.head))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Set an environment variable",
                example: r#"export-env { let-env SPAM = 'eggs' }"#,
                result: Some(Value::Nothing {
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Set an environment variable and examine its value",
                example: r#"export-env { let-env SPAM = 'eggs' }; $env.SPAM"#,
                result: Some(Value::string("eggs", Span::test_data())),
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

        test_examples(ExportEnv {})
    }
}
