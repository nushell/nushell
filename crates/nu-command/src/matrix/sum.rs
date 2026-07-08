use crate::matrix::MatrixValue;
use ndarray::Axis;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MatrixSum;

impl Command for MatrixSum {
    fn name(&self) -> &str {
        "matrix sum"
    }

    fn signature(&self) -> Signature {
        Signature::build("matrix sum")
            .input_output_types(vec![
                (Type::Custom("matrix".into()), Type::Float),
                (Type::Custom("matrix".into()), Type::Custom("matrix".into())),
            ])
            .named(
                "axis",
                SyntaxShape::Int,
                "The axis to sum along (0-based).",
                Some('a'),
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Sum all elements of a matrix, or sum along an axis."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["total", "add"]
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
                let result = matrix.array.sum_axis(Axis(axis));
                Ok(MatrixValue::new(result)
                    .into_value(head)
                    .into_pipeline_data())
            }
            None => {
                let total: f64 = matrix.array.sum();
                Ok(Value::float(total, head).into_pipeline_data())
            }
        }
    }

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                description: "Sum all elements of a 2x2 matrix",
                example: "[[1 2] [3 4]] | into matrix | matrix sum",
                result: Some(Value::test_float(10.0)),
            },
            Example {
                description: "Sum along rows (axis 0)",
                example: "[[1 2] [3 4]] | into matrix | matrix sum --axis 0 | matrix into-nu | to nuon",
                result: Some(Value::test_string("[[4.0, 6.0]]")),
            },
            Example {
                description: "Sum along columns (axis 1)",
                example: "[[1 2] [3 4]] | into matrix | matrix sum --axis 1 | matrix into-nu | to nuon",
                result: Some(Value::test_string("[[3.0, 7.0]]")),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(MatrixSum)
    }
}
