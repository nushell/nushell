use nu_engine::eval_block;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Span,
    SyntaxShape, Value,
};

#[derive(Clone)]
pub struct SkipWhile;

impl Command for SkipWhile {
    fn name(&self) -> &str {
        "skip while"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "predicate",
                SyntaxShape::RowCondition,
                "the predicate that skipped element must match",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Skip elements of the input while a predicate is true."
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Skip while the element is negative",
            example: "echo [-2 0 2 -1] | skip while $it < 0",
            result: Some(Value::List {
                vals: vec![Value::from(0), Value::from(2), Value::from(-1)],
                span: Span::unknown(),
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
        let predicate = &call.positional[0];
        let span = call.head;

        let block_id = predicate
            .as_row_condition_block()
            .ok_or_else(|| ShellError::TypeMismatch("expected row condition".to_owned(), span))?;

        let block = engine_state.get_block(block_id).clone();
        let var_id = block.signature.get_positional(0).and_then(|arg| arg.var_id);
        let mut stack = stack.collect_captures(&block.captures);

        let ctrlc = engine_state.ctrlc.clone();
        let engine_state = engine_state.clone();

        Ok(input
            .into_iter()
            .skip_while(move |value| {
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

        test_examples(SkipWhile)
    }
}
