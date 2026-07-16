use crate::matrix::MatrixValue;
use crate::matrix::value::positive_dim;
use ndarray::ArrayD;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MatrixIdentity;

impl Command for MatrixIdentity {
    fn name(&self) -> &str {
        "matrix identity"
    }

    fn signature(&self) -> Signature {
        Signature::build("matrix identity")
            .input_output_types(vec![(Type::Nothing, Type::Custom("matrix".into()))])
            .required(
                "size",
                SyntaxShape::Int,
                "The size of the square identity matrix.",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Create an identity matrix of the given size."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["eye", "unit"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let size = positive_dim(call.req::<i64>(engine_state, stack, 0)?, head)?;

        let mut array = ArrayD::zeros(vec![size, size]);
        for i in 0..size {
            array[[i, i]] = 1.0;
        }
        Ok(MatrixValue::new(array)
            .into_value(head)
            .into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                description: "Create a 3x3 identity matrix",
                example: "matrix identity 3 | matrix into-nu | to nuon",
                result: Some(Value::test_string(
                    "[[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]",
                )),
            },
            Example {
                description: "Create a 2x2 identity matrix",
                example: "matrix identity 2 | matrix into-nu | to nuon",
                result: Some(Value::test_string("[[1.0, 0.0], [0.0, 1.0]]")),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(MatrixIdentity)
    }
}
