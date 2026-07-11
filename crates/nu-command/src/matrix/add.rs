use crate::matrix::MatrixValue;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MatrixAdd;

impl Command for MatrixAdd {
    fn name(&self) -> &str {
        "matrix add"
    }

    fn signature(&self) -> Signature {
        Signature::build("matrix add")
            .input_output_types(vec![(
                Type::Custom("matrix".into()),
                Type::Custom("matrix".into()),
            )])
            .required(
                "other",
                SyntaxShape::Any,
                "The other matrix or scalar to add.",
            )
            .switch(
                "broadcast",
                "Enable broadcasting to allow compatible shapes",
                Some('b'),
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Add a matrix or scalar to a matrix."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["plus", "sum"]
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
            matrix.elementwise_binary(other, broadcast, head, |a, b| a + b, |a, s| a + s)?;

        Ok(MatrixValue::new(result)
            .into_value(head)
            .into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                description: "Add a scalar to a matrix",
                example: "matrix zeros 2 2 | matrix add 5 | matrix into-nu | to nuon",
                result: Some(Value::test_string("[[5.0, 5.0], [5.0, 5.0]]")),
            },
            Example {
                description: "Add two matrices element-wise",
                example: "matrix identity 2 | matrix add (matrix identity 2) | matrix into-nu | to nuon",
                result: Some(Value::test_string("[[2.0, 0.0], [0.0, 2.0]]")),
            },
            Example {
                description: "Add with broadcasting a row vector",
                example: "matrix zeros 2 3 | matrix add --broadcast ([[1.0 2.0 3.0]] | into matrix) | matrix into-nu | to nuon",
                result: Some(Value::test_string("[[1.0, 2.0, 3.0], [1.0, 2.0, 3.0]]")),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(MatrixAdd)
    }
}
