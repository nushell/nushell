use crate::matrix::MatrixValue;
use ndarray::Axis;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MatrixMax;

impl Command for MatrixMax {
    fn name(&self) -> &str {
        "matrix max"
    }

    fn signature(&self) -> Signature {
        Signature::build("matrix max")
            .input_output_types(vec![
                (Type::Custom("matrix".into()), Type::Float),
                (Type::Custom("matrix".into()), Type::Custom("matrix".into())),
            ])
            .named(
                "axis",
                SyntaxShape::Int,
                "The axis to find the max along (0-based).",
                Some('a'),
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Find the maximum value in a matrix, or max along an axis."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["maximum"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let axis: Option<i64> = call.get_flag(engine_state, stack, "axis")?;
        let matrix = MatrixValue::from_value(&input.into_value(head)?)?;

        match axis {
            Some(axis) => {
                let axis = axis as usize;
                if axis >= matrix.array.ndim() {
                    return Err(ShellError::Generic(
                        nu_protocol::shell_error::generic::GenericError::new(
                            "Invalid axis",
                            format!(
                                "axis {} is out of bounds for a {}-dimensional array",
                                axis,
                                matrix.array.ndim()
                            ),
                            head,
                        ),
                    ));
                }
                let result = matrix.array.map_axis(Axis(axis), |view| {
                    view.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
                });
                Ok(MatrixValue::new(result)
                    .into_value(head)
                    .into_pipeline_data())
            }
            None => {
                let max_val: f64 = matrix
                    .array
                    .iter()
                    .cloned()
                    .fold(f64::NEG_INFINITY, f64::max);
                Ok(Value::float(max_val, head).into_pipeline_data())
            }
        }
    }

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                description: "Find the maximum element in a matrix",
                example: "[[1 2] [3 4]] | into matrix | matrix max",
                result: Some(Value::test_float(4.0)),
            },
            Example {
                description: "Find max along rows (axis 0)",
                example: "[[1 2] [3 4]] | into matrix | matrix max --axis 0 | matrix into-nu | to nuon",
                result: Some(Value::test_string("[[3.0, 4.0]]")),
            },
            Example {
                description: "Find max along columns (axis 1)",
                example: "[[1 2] [3 4]] | into matrix | matrix max --axis 1 | matrix into-nu | to nuon",
                result: Some(Value::test_string("[[2.0, 4.0]]")),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(MatrixMax)
    }
}
