use crate::matrix::MatrixValue;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MatrixGetCol;

impl Command for MatrixGetCol {
    fn name(&self) -> &str {
        "matrix get-col"
    }

    fn signature(&self) -> Signature {
        Signature::build("matrix get-col")
            .input_output_types(vec![(
                Type::Custom("matrix".into()),
                Type::List(Box::new(Type::Float)),
            )])
            .required(
                "index",
                SyntaxShape::Int,
                "The column index to extract (0-based).",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Extract a column from a 2D matrix."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["column"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let index: usize = call.req::<i64>(engine_state, stack, 0)? as usize;
        let matrix = MatrixValue::from_value(&input.into_value(head)?)?;

        if matrix.array.ndim() != 2 {
            return Err(ShellError::Generic(
                nu_protocol::shell_error::generic::GenericError::new(
                    "Invalid dimensions",
                    format!(
                        "get-col requires a 2D matrix, got {} dimensions",
                        matrix.array.ndim()
                    ),
                    head,
                ),
            ));
        }

        let ncols = matrix.array.shape()[1];
        if index >= ncols {
            return Err(ShellError::Generic(
                nu_protocol::shell_error::generic::GenericError::new(
                    "Index out of bounds",
                    format!(
                        "column index {} is out of bounds, matrix has {} columns",
                        index, ncols
                    ),
                    head,
                ),
            ));
        }

        let col = matrix.array.index_axis(ndarray::Axis(1), index);
        let vals: Vec<Value> = col.iter().map(|v| Value::float(*v, head)).collect();
        Ok(Value::list(vals, head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                description: "Get the first column of a 2x3 matrix",
                example: "matrix zeros 2 3 | matrix set-col 0 [1.0 2.0] | matrix get-col 0 | to nuon",
                result: Some(Value::test_string("[1.0, 2.0]")),
            },
            Example {
                description: "Get the second column of a 2x2 identity matrix",
                example: "matrix identity 2 | matrix get-col 1 | to nuon",
                result: Some(Value::test_string("[0.0, 1.0]")),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(MatrixGetCol)
    }
}
