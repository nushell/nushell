use nu_engine::eval_block;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Example, PipelineData, Signature, SyntaxShape, Value};

#[derive(Clone)]
pub struct Collect;

impl Command for Collect {
    fn name(&self) -> &str {
        "collect"
    }

    fn signature(&self) -> Signature {
        Signature::build("collect").required(
            "block",
            SyntaxShape::Block(Some(vec![SyntaxShape::Any])),
            "the block to run once the stream is collected",
        )
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
        let block_id = call.positional[0]
            .as_block()
            .expect("internal error: expected block");

        let block = engine_state.get_block(block_id).clone();
        let mut stack = stack.collect_captures(&block.captures);

        let input: Value = input.into_value(call.head);

        if let Some(var) = block.signature.get_positional(0) {
            if let Some(var_id) = &var.var_id {
                stack.add_var(*var_id, input);
            }
        }

        eval_block(
            engine_state,
            &mut stack,
            &block,
            PipelineData::new(call.head),
        )
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
