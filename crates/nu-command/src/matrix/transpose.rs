use crate::matrix::MatrixValue;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MatrixTranspose;

impl Command for MatrixTranspose {
    fn name(&self) -> &str {
        "matrix transpose"
    }

    fn signature(&self) -> Signature {
        Signature::build("matrix transpose")
            .input_output_types(vec![(
                Type::Custom("matrix".into()),
                Type::Custom("matrix".into()),
            )])
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Transpose a matrix (swap rows and columns). For n-dimensional arrays, reverses all axes."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["swap", "flip"]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let matrix = MatrixValue::from_value(&input.into_value(head)?)?;

        let result = if matrix.array.ndim() == 2 {
            matrix.array.t().to_owned()
        } else {
            let axes: Vec<usize> = (0..matrix.array.ndim()).rev().collect();
            matrix.array.permuted_axes(ndarray::IxDyn(&axes)).to_owned()
        };

        Ok(MatrixValue::new(result)
            .into_value(head)
            .into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                description: "Transpose a 2x3 matrix to a 3x2 matrix",
                example: "[[1 2 3] [4 5 6]] | into matrix | matrix transpose | matrix into-nu | to nuon",
                result: Some(Value::test_string("[[1.0, 4.0], [2.0, 5.0], [3.0, 6.0]]")),
            },
            Example {
                description: "Transpose an identity matrix (result is the same)",
                example: "matrix identity 2 | matrix transpose | matrix into-nu | to nuon",
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
        nu_test_support::test().examples(MatrixTranspose)
    }
}
