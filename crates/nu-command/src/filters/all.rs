use nu_engine::eval_block;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape,
};

#[derive(Clone)]
pub struct All;

impl Command for All {
    fn name(&self) -> &str {
        "all?"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "predicate",
                SyntaxShape::RowCondition,
                "the predicate that must match",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Test if every element of the input matches a predicate."
    }

    fn examples(&self) -> Vec<Example> {
        use nu_protocol::Value;

        vec![
            Example {
                description: "Find if services are running",
                example: "echo [[status]; [UP] [UP]] | all? status == UP",
                result: Some(Value::from(true)),
            },
            Example {
                description: "Check that all values are even",
                example: "echo [2 4 6 8] | all? ($it mod 2) == 0",
                result: Some(Value::from(true)),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let predicate = &call.positional[0];
        let block_id = predicate
            .as_row_condition_block()
            .ok_or_else(|| ShellError::InternalError("Expected row condition".to_owned()))?;

        let span = call.head;

        let block = engine_state.get_block(block_id);
        let var_id = block.signature.get_positional(0).and_then(|arg| arg.var_id);
        let mut stack = stack.collect_captures(&block.captures);

        let ctrlc = engine_state.ctrlc.clone();
        let engine_state = engine_state.clone();

        Ok(input
            .into_interruptible_iter(ctrlc)
            .all(move |value| {
                if let Some(var_id) = var_id {
                    stack.add_var(var_id, value);
                }

                eval_block(&engine_state, &mut stack, block, PipelineData::new(span))
                    .map_or(false, |pipeline_data| {
                        pipeline_data.into_value(span).is_true()
                    })
            })
            .into_pipeline_data())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(All)
    }
}
