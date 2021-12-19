use nu_engine::eval_block;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Any;

impl Command for Any {
    fn name(&self) -> &str {
        "any?"
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
        "Tests if any element of the input matches a predicate."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Find if a service is not running",
                example: "echo [[status]; [UP] [DOWN] [UP]] | any? status == DOWN",
                result: Some(Value::test_bool(true)),
            },
            Example {
                description: "Check if any of the values is odd",
                example: "echo [2 4 1 6 8] | any? ($it mod 2) == 1",
                result: Some(Value::test_bool(true)),
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
        let span = call.head;

        let block_id = predicate
            .as_row_condition_block()
            .ok_or_else(|| ShellError::TypeMismatch("expected row condition".to_owned(), span))?;

        let block = engine_state.get_block(block_id);
        let var_id = block.signature.get_positional(0).and_then(|arg| arg.var_id);
        let mut stack = stack.collect_captures(&block.captures);

        let ctrlc = engine_state.ctrlc.clone();
        let engine_state = engine_state.clone();

        Ok(Value::Bool {
            val: input.into_interruptible_iter(ctrlc).any(move |value| {
                if let Some(var_id) = var_id {
                    stack.add_var(var_id, value);
                }

                eval_block(&engine_state, &mut stack, block, PipelineData::new(span))
                    .map_or(false, |pipeline_data| {
                        pipeline_data.into_value(span).is_true()
                    })
            }),
            span,
        }
        .into_pipeline_data())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Any)
    }
}
