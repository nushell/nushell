use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};

/// Source a file for environment variables.
#[derive(Clone)]
pub struct Source;

impl Command for Source {
    fn name(&self) -> &str {
        "source"
    }

    fn signature(&self) -> Signature {
        Signature::build("source")
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

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // Note: this hidden positional is the block_id that corresponded to the 0th position
        // it is put here by the parser
        let block_id: i64 = call.req(engine_state, stack, 1)?;

        let block = engine_state.get_block(block_id as usize).clone();
        eval_block(engine_state, stack, &block, input)
    }
}
