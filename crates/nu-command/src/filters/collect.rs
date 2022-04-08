use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{CaptureBlock, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Collect;

impl Command for Collect {
    fn name(&self) -> &str {
        "collect"
    }

    fn signature(&self) -> Signature {
        Signature::build("collect")
            .required(
                "block",
                SyntaxShape::Block(Some(vec![SyntaxShape::Any])),
                "the block to run once the stream is collected",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Collect the stream and pass it to a block."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let capture_block: CaptureBlock = call.req(engine_state, stack, 0)?;

        let block = engine_state.get_block(capture_block.block_id).clone();
        let mut stack = stack.captures_to_stack(&capture_block.captures);

        let metadata = input.metadata();
        let input: Value = input.into_value(call.head);

        if let Some(var) = block.signature.get_positional(0) {
            if let Some(var_id) = &var.var_id {
                stack.add_var(*var_id, input.clone());
            }
        }

        eval_block(
            engine_state,
            &mut stack,
            &block,
            input.into_pipeline_data(),
            call.redirect_stdout,
            call.redirect_stderr,
        )
        .map(|x| x.set_metadata(metadata))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Use the second value in the stream",
            example: "echo 1 2 3 | collect { |x| echo $x.1 }",
            result: Some(Value::test_int(2)),
        }]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Collect {})
    }
}
