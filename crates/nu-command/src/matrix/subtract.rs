use crate::matrix::MatrixValue;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MatrixSubtract;

impl Command for MatrixSubtract {
    fn name(&self) -> &str {
        "matrix subtract"
    }

    fn signature(&self) -> Signature {
        Signature::build("matrix subtract")
            .input_output_types(vec![(
                Type::Custom("matrix".into()),
                Type::Custom("matrix".into()),
            )])
            .required(
                "other",
                SyntaxShape::Any,
                "The other matrix or scalar to subtract.",
            )
            .switch(
                "broadcast",
                "Enable broadcasting to allow compatible shapes",
                Some('b'),
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Subtract a matrix or scalar from a matrix."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["minus", "difference"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let other: Value = call.req(engine_state, stack, 0)?;
        let broadcast = call.has_flag(engine_state, stack, "broadcast")?;
        let matrix = MatrixValue::from_value(&input.into_value(head)?)?;

        let result =
            matrix.elementwise_binary(other, broadcast, head, |a, b| a - b, |a, s| a - s)?;

        Ok(MatrixValue::new(result)
            .into_value(head)
            .into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                description: "Subtract a scalar from a matrix",
                example: "matrix zeros 2 2 | matrix add 5 | matrix subtract 2 | matrix into-nu | to nuon",
                result: Some(Value::test_string("[[3.0, 3.0], [3.0, 3.0]]")),
            },
            Example {
                description: "Subtract two matrices element-wise",
                example: "matrix identity 2 | matrix add 5 | matrix subtract (matrix identity 2) | matrix into-nu | to nuon",
                result: Some(Value::test_string("[[5.0, 5.0], [5.0, 5.0]]")),
            },
            Example {
                description: "Subtract with broadcasting a row vector",
                example: "matrix zeros 2 3 | matrix add 5 | matrix subtract --broadcast ([[1.0 2.0 3.0]] | into matrix) | matrix into-nu | to nuon",
                result: Some(Value::test_string("[[4.0, 3.0, 2.0], [4.0, 3.0, 2.0]]")),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(MatrixSubtract)
    }
}
