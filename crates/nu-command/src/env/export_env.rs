use nu_engine::{command_prelude::*, get_eval_block, redirect_env};
use nu_protocol::engine::CommandType;

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

    fn description(&self) -> &str {
        "Run a block and preserve its environment in a current scope."
    }

    fn extra_description(&self) -> &str {
        "This command is a parser keyword. For details, check:
  https://www.nushell.sh/book/thinking_in_nu.html"
    }

    fn command_type(&self) -> CommandType {
        CommandType::Keyword
    }

    fn requires_ast_for_arguments(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        caller_stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let arg = call.positional_nth(caller_stack, 0).ok_or_else(|| {
            ShellError::MissingParameter {
                param_name: "block".into(),
                span: head,
            }
        })?;
        let block_id = arg.as_block().ok_or_else(|| ShellError::TypeMismatch {
            err_message: "expected block".into(),
            span: arg.span,
        })?;

        let block = engine_state.get_block(block_id);
        let mut callee_stack = caller_stack
            .gather_captures(engine_state, &block.captures)
            .reset_pipes();

        let eval_block = get_eval_block(engine_state);

        // Run the block (discard the result)
        let _ = eval_block(engine_state, &mut callee_stack, block, input)?;

        // Merge the block's environment to the current stack
        redirect_env(engine_state, caller_stack, &callee_stack);

        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Set an environment variable.",
                example: "export-env { $env.SPAM = 'eggs' }",
                result: Some(Value::nothing(Span::test_data())),
            },
            Example {
                description: "Set an environment variable and examine its value.",
                example: "export-env { $env.SPAM = 'eggs' }; $env.SPAM",
                result: Some(Value::test_string("eggs")),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(ExportEnv)
    }

    #[test]
    fn test_missing_block_does_not_panic() -> nu_test_support::Result {
        // Regression: previously, `export-env` was given a non-block argument
        // (e.g. a closure) and panicked with "internal error: missing block".
        // See https://github.com/nushell/nushell/issues/13037.
        nu_test_support::test()
            .run("export-env { |x| $x }")
            .expect_error()
    }
}
