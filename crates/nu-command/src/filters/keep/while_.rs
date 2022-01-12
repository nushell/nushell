use nu_engine::{eval_block, CallExt};
use nu_protocol::{
    ast::Call,
    engine::{CaptureBlock, Command, EngineState, Stack},
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Span,
    SyntaxShape, Value,
};

#[derive(Clone)]
pub struct KeepWhile;

impl Command for KeepWhile {
    fn name(&self) -> &str {
        "keep while"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "predicate",
                SyntaxShape::RowCondition,
                "the predicate that kept element must not match",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Keep elements of the input while a predicate is true."
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Keep while the element is negative",
            example: "echo [-1 -2 9 1] | keep while $it < 0",
            result: Some(Value::List {
                vals: vec![Value::test_int(-1), Value::test_int(-2)],
                span: Span::test_data(),
            }),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;

        let capture_block: CaptureBlock = call.req(engine_state, stack, 0)?;

        let block = engine_state.get_block(capture_block.block_id).clone();
        let var_id = block.signature.get_positional(0).and_then(|arg| arg.var_id);

        let mut stack = stack.captures_to_stack(&capture_block.captures);

        let ctrlc = engine_state.ctrlc.clone();
        let engine_state = engine_state.clone();

        Ok(input
            .into_iter()
            .take_while(move |value| {
                if let Some(var_id) = var_id {
                    stack.add_var(var_id, value.clone());
                }

                eval_block(&engine_state, &mut stack, &block, PipelineData::new(span))
                    .map_or(false, |pipeline_data| {
                        pipeline_data.into_value(span).is_true()
                    })
            })
            .into_pipeline_data(ctrlc))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(KeepWhile)
    }
}
