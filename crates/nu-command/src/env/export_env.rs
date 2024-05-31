use nu_engine::{command_prelude::*, get_eval_block, redirect_env};

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
                "The block to run to set the environment.",
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
    ) -> Result<PipelineData, ShellError> {
        let block_id = call
            .positional_nth(0)
            .expect("checked through parser")
            .as_block()
            .expect("internal error: missing block");

        let block = engine_state.get_block(block_id);
        let mut callee_stack = caller_stack
            .gather_captures(engine_state, &block.captures)
            .reset_pipes();

        let eval_block = get_eval_block(engine_state);

        let _ = eval_block(engine_state, &mut callee_stack, block, input);

        redirect_env(engine_state, caller_stack, &callee_stack);

        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Set an environment variable",
                example: r#"export-env { $env.SPAM = 'eggs' }"#,
                result: Some(Value::nothing(Span::test_data())),
            },
            Example {
                description: "Set an environment variable and examine its value",
                example: r#"export-env { $env.SPAM = 'eggs' }; $env.SPAM"#,
                result: Some(Value::test_string("eggs")),
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
