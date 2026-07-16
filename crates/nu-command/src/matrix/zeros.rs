use crate::matrix::MatrixValue;
use crate::matrix::value::positive_dim;
use ndarray::ArrayD;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MatrixZeros;

impl Command for MatrixZeros {
    fn name(&self) -> &str {
        "matrix zeros"
    }

    fn signature(&self) -> Signature {
        Signature::build("matrix zeros")
            .input_output_types(vec![(Type::Nothing, Type::Custom("matrix".into()))])
            .required(
                "dimensions",
                SyntaxShape::Int,
                "The dimensions of the zero matrix (e.g., 3 4 for a 3x4 matrix).",
            )
            .rest(
                "more_dimensions",
                SyntaxShape::Int,
                "Additional dimensions for n-dimensional arrays.",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Create a matrix filled with zeros."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["zeroes"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let first_dim: i64 = call.req(engine_state, stack, 0)?;
        let rest_dims: Vec<Value> = call.rest(engine_state, stack, 1)?;

        let mut shape = vec![positive_dim(first_dim, head)?];
        for v in rest_dims {
            let d = v.as_int().map_err(|_| {
                ShellError::Generic(nu_protocol::shell_error::generic::GenericError::new(
                    "Invalid dimensions",
                    "dimensions must be positive integers",
                    head,
                ))
            })?;
            shape.push(positive_dim(d, head)?);
        }

        let array = ArrayD::zeros(shape);
        Ok(MatrixValue::new(array)
            .into_value(head)
            .into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                description: "Create a 3x4 matrix of zeros",
                example: "matrix zeros 3 4 | matrix into-nu | to nuon",
                result: Some(Value::test_string(
                    "[[0.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0]]",
                )),
            },
            Example {
                description: "Create a 2x2 matrix of zeros",
                example: "matrix zeros 2 2 | matrix into-nu | to nuon",
                result: Some(Value::test_string("[[0.0, 0.0], [0.0, 0.0]]")),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(MatrixZeros)
    }
}
