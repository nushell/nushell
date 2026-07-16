use crate::matrix::MatrixValue;
use crate::matrix::value::values_to_f64s;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MatrixSetCol;

impl Command for MatrixSetCol {
    fn name(&self) -> &str {
        "matrix set-col"
    }

    fn signature(&self) -> Signature {
        Signature::build("matrix set-col")
            .input_output_types(vec![(
                Type::Custom("matrix".into()),
                Type::Custom("matrix".into()),
            )])
            .required(
                "index",
                SyntaxShape::Int,
                "The column index to replace (0-based).",
            )
            .required(
                "replacement",
                SyntaxShape::List(Box::new(SyntaxShape::Number)),
                "The new column values as a list of numbers.",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Replace a column in a 2D matrix."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["column", "replace"]
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

        if matrix.array.ndim() != 2 {
            return Err(ShellError::Generic(
                nu_protocol::shell_error::generic::GenericError::new(
                    "Invalid dimensions",
                    format!(
                        "set-col requires a 2D matrix, got {} dimensions",
                        matrix.array.ndim()
                    ),
                    head,
                ),
            ));
        }

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

        let nrows = matrix.array.shape()[0];
        let ncols = matrix.array.shape()[1];

        if replacement_vals.len() != nrows {
            return Err(ShellError::Generic(
                nu_protocol::shell_error::generic::GenericError::new(
                    "Size mismatch",
                    format!(
                        "replacement has {} elements, but column has {} rows",
                        replacement_vals.len(),
                        nrows
                    ),
                    head,
                ),
            ));
        }

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

        let floats = values_to_f64s(&replacement_vals, head)?;

        let mut new_array = matrix.array;
        for (i, &val) in floats.iter().enumerate() {
            new_array[[i, index]] = val;
        }

        Ok(MatrixValue::new(new_array)
            .into_value(head)
            .into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                description: "Replace the first column of a 2x3 matrix",
                example: "matrix zeros 2 3 | matrix set-col 0 [1.0 2.0] | matrix into-nu | to nuon",
                result: Some(Value::test_string("[[1.0, 0.0, 0.0], [2.0, 0.0, 0.0]]")),
            },
            Example {
                description: "Replace the second column of a 2x2 matrix",
                example: "matrix zeros 2 2 | matrix set-col 1 [4.0 5.0] | matrix into-nu | to nuon",
                result: Some(Value::test_string("[[0.0, 4.0], [0.0, 5.0]]")),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(MatrixSetCol)
    }
}
