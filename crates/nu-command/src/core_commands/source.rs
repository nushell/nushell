use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{PipelineData, ShellError, Signature, SyntaxShape, Value};

/// Source a file for environment variables.
#[derive(Clone)]
pub struct Source;

impl Command for Source {
    fn name(&self) -> &str {
        "source"
    }

    fn signature(&self) -> Signature {
        Signature::build("source").required(
            "filename",
            SyntaxShape::Filepath,
            "the filepath to the script file to source",
        )
    }

    fn usage(&self) -> &str {
        "Runs a script file in the current context."
    }

    fn run(
        &self,
        context: &EvaluationContext,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // Note: this hidden positional is the block_id that corresponded to the 0th position
        // it is put here by the parser
        let block_id: i64 = call.req(context, 1)?;

        let block = context.engine_state.get_block(block_id as usize).clone();
        eval_block(context, &block, input)
    }
}
