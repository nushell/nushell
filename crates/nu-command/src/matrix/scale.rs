use crate::matrix::MatrixValue;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MatrixScale;

impl Command for MatrixScale {
    fn name(&self) -> &str {
        "matrix scale"
    }

    fn signature(&self) -> Signature {
        Signature::build("matrix scale")
            .input_output_types(vec![(
                Type::Custom("matrix".into()),
                Type::Custom("matrix".into()),
            )])
            .required(
                "scalar",
                SyntaxShape::Number,
                "The scalar to multiply each element by.",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Multiply all elements of a matrix by a scalar."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["multiply", "scalar"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let scalar: Value = call.req(engine_state, stack, 0)?;
        let matrix = MatrixValue::from_value(&input.into_value(head)?)?;

        let factor = match scalar {
            Value::Int { val, .. } => val as f64,
            Value::Float { val, .. } => val,
            _ => {
                return Err(ShellError::Generic(
                    nu_protocol::shell_error::generic::GenericError::new(
                        "Invalid argument",
                        "expected a number",
                        head,
                    ),
                ));
            }
        };

        let result = matrix.array * factor;
        Ok(MatrixValue::new(result)
            .into_value(head)
            .into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                description: "Scale a matrix by an integer",
                example: "matrix identity 2 | matrix scale 3 | matrix into-nu | to nuon",
                result: Some(Value::test_string("[[3.0, 0.0], [0.0, 3.0]]")),
            },
            Example {
                description: "Scale a matrix by a float",
                example: "matrix identity 2 | matrix scale 0.5 | matrix into-nu | to nuon",
                result: Some(Value::test_string("[[0.5, 0.0], [0.0, 0.5]]")),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(MatrixScale)
    }
}
