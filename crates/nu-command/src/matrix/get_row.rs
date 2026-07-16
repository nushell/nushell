use crate::matrix::MatrixValue;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MatrixGetRow;

impl Command for MatrixGetRow {
    fn name(&self) -> &str {
        "matrix get-row"
    }

    fn signature(&self) -> Signature {
        Signature::build("matrix get-row")
            .input_output_types(vec![(
                Type::Custom("matrix".into()),
                Type::List(Box::new(Type::Float)),
            )])
            .required(
                "index",
                SyntaxShape::Int,
                "The row index to extract (0-based).",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Extract a row from a matrix."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![]
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

        if index >= matrix.array.shape()[0] {
            return Err(ShellError::Generic(
                nu_protocol::shell_error::generic::GenericError::new(
                    "Index out of bounds",
                    format!(
                        "row index {} is out of bounds, matrix has {} rows",
                        index,
                        matrix.array.shape()[0]
                    ),
                    head,
                ),
            ));
        }

        let row = matrix.array.index_axis(ndarray::Axis(0), index);
        let vals: Vec<Value> = row.iter().map(|v| Value::float(*v, head)).collect();
        Ok(Value::list(vals, head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                description: "Get the first row of a 2x3 matrix",
                example: "matrix zeros 2 3 | matrix set-row 0 [1.0 2.0 3.0] | matrix get-row 0 | to nuon",
                result: Some(Value::test_string("[1.0, 2.0, 3.0]")),
            },
            Example {
                description: "Get the second row of a 2x2 identity matrix",
                example: "matrix identity 2 | matrix get-row 1 | to nuon",
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
        nu_test_support::test().examples(MatrixGetRow)
    }
}
