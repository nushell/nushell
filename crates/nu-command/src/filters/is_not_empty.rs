use crate::filters::empty::empty;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct IsNotEmpty;

impl Command for IsNotEmpty {
    fn name(&self) -> &str {
        "is-not-empty"
    }

    fn signature(&self) -> Signature {
        Signature::build("is-not-empty")
            .input_output_types(vec![(Type::Any, Type::Bool)])
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "The names of the columns to check emptiness.",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Check for non-empty values."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // Call the same `empty` function but negate the result
        empty(engine_state, stack, call, input, true)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Check if a string is empty",
                example: "'' | is-not-empty",
                result: Some(Value::test_bool(false)),
            },
            Example {
                description: "Check if a list is empty",
                example: "[] | is-not-empty",
                result: Some(Value::test_bool(false)),
            },
            Example {
                // TODO: revisit empty cell path semantics for a record.
                description: "Check if more than one column are empty",
                example: "[[meal size]; [arepa small] [taco '']] | is-not-empty meal size",
                result: Some(Value::test_bool(true)),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(IsNotEmpty {})
    }
}
