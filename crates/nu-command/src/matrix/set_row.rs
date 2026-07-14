use crate::matrix::MatrixValue;
use crate::matrix::value::values_to_f64s;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MatrixSetRow;

impl Command for MatrixSetRow {
    fn name(&self) -> &str {
        "matrix set-row"
    }

    fn signature(&self) -> Signature {
        Signature::build("matrix set-row")
            .input_output_types(vec![(
                Type::Custom("matrix".into()),
                Type::Custom("matrix".into()),
            )])
            .required(
                "index",
                SyntaxShape::Int,
                "The row index to replace (0-based).",
            )
            .required(
                "replacement",
                SyntaxShape::List(Box::new(SyntaxShape::Number)),
                "The new row values as a list of numbers.",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Replace a row in a matrix."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["replace"]
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
        let replacement: Value = call.req(engine_state, stack, 1)?;
        let matrix = MatrixValue::from_value(&input.into_value(head)?)?;

        let replacement_vals = match replacement {
            Value::List { vals, .. } => vals,
            _ => {
                return Err(ShellError::Generic(
                    nu_protocol::shell_error::generic::GenericError::new(
                        "Invalid replacement",
                        "expected a list of numbers",
                        head,
                    ),
                ));
            }
        };

        let ncols = if matrix.array.ndim() >= 2 {
            matrix.array.shape()[1]
        } else {
            1
        };
        if replacement_vals.len() != ncols {
            return Err(ShellError::Generic(
                nu_protocol::shell_error::generic::GenericError::new(
                    "Size mismatch",
                    format!(
                        "replacement has {} elements, but row has {} columns",
                        replacement_vals.len(),
                        ncols
                    ),
                    head,
                ),
            ));
        }

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

        let floats = values_to_f64s(&replacement_vals, head)?;

        let mut new_array = matrix.array;
        for (j, &val) in floats.iter().enumerate() {
            if new_array.ndim() == 1 {
                new_array[[index]] = val;
            } else {
                new_array[[index, j]] = val;
            }
        }

        Ok(MatrixValue::new(new_array)
            .into_value(head)
            .into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                description: "Replace the first row of a 2x3 matrix",
                example: "matrix zeros 2 3 | matrix set-row 0 [1.0 2.0 3.0] | matrix into-nu | to nuon",
                result: Some(Value::test_string("[[1.0, 2.0, 3.0], [0.0, 0.0, 0.0]]")),
            },
            Example {
                description: "Replace the second row of a 2x2 matrix",
                example: "matrix zeros 2 2 | matrix set-row 1 [4.0 5.0] | matrix into-nu | to nuon",
                result: Some(Value::test_string("[[0.0, 0.0], [4.0, 5.0]]")),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(MatrixSetRow)
    }
}
