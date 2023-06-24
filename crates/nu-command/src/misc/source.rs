use nu_engine::{eval_block_with_early_return, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Type};

/// Source a file for environment variables.
#[derive(Clone)]
pub struct Source;

impl Command for Source {
    fn name(&self) -> &str {
        "source"
    }

    fn signature(&self) -> Signature {
        Signature::build("source")
            .input_output_types(vec![(Type::Any, Type::Any)])
            .required(
                "filename",
                SyntaxShape::Filepath,
                "the filepath to the script file to source",
            )
            .category(Category::Core)
    }

    fn usage(&self) -> &str {
        "Runs a script file in the current context."
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
    ) -> Result<PipelineData, ShellError> {
        // Note: this hidden positional is the block_id that corresponded to the 0th position
        // it is put here by the parser
        let block_id: i64 = call.req_parser_info(engine_state, stack, "block_id")?;

        let block = engine_state.get_block(block_id as usize).clone();
        eval_block_with_early_return(
            engine_state,
            stack,
            &block,
            input,
            call.redirect_stdout,
            call.redirect_stderr,
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Runs foo.nu in the current context",
                example: r#"source foo.nu"#,
                result: None,
            },
            Example {
                description: "Runs foo.nu in current context and call the command defined, suppose foo.nu has content: `def say-hi [] { echo 'Hi!' }`",
                example: r#"source ./foo.nu; say-hi"#,
                result: None,
            },
        ]
    }
}
