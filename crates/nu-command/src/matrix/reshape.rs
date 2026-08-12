use crate::matrix::MatrixValue;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MatrixReshape;

impl Command for MatrixReshape {
    fn name(&self) -> &str {
        "matrix reshape"
    }

    fn signature(&self) -> Signature {
        Signature::build("matrix reshape")
            .input_output_types(vec![(
                Type::Custom("matrix".into()),
                Type::Custom("matrix".into()),
            )])
            .optional(
                "dimensions",
                SyntaxShape::Int,
                "The new dimensions (e.g., 2 3 for a 2x3 matrix). Required unless --flatten is used.",
            )
            .rest(
                "more_dimensions",
                SyntaxShape::Int,
                "Additional dimensions for n-dimensional reshaping.",
            )
            .switch("flatten", "Flatten the matrix to a 1D vector", Some('f'))
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Change the dimensions of a matrix."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["dimensions", "flatten"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let flatten = call.has_flag(engine_state, stack, "flatten")?;
        let matrix = MatrixValue::from_value(&input.into_value(head)?)?;

        let result = if flatten {
            let total: usize = matrix.array.len();
            matrix
                .array
                .into_shape_with_order(ndarray::IxDyn(&[total]))
                .map_err(|e| {
                    ShellError::Generic(nu_protocol::shell_error::generic::GenericError::new(
                        "Reshape error",
                        e.to_string(),
                        head,
                    ))
                })?
        } else {
            let first_dim: Option<i64> = call.opt(engine_state, stack, 0)?;
            let first_dim = first_dim.ok_or_else(|| {
                ShellError::Generic(nu_protocol::shell_error::generic::GenericError::new(
                    "Missing dimensions",
                    "at least one dimension is required, or use --flatten",
                    head,
                ))
            })? as usize;
            let rest_dims: Vec<Value> = call.rest(engine_state, stack, 1)?;

            let mut shape = vec![first_dim];
            for v in rest_dims {
                match v.as_int() {
                    Ok(d) if d > 0 => shape.push(d as usize),
                    _ => {
                        return Err(ShellError::Generic(
                            nu_protocol::shell_error::generic::GenericError::new(
                                "Invalid dimensions",
                                "dimensions must be positive integers",
                                head,
                            ),
                        ));
                    }
                }
            }

            for &d in &shape {
                if d == 0 {
                    return Err(ShellError::Generic(
                        nu_protocol::shell_error::generic::GenericError::new(
                            "Invalid dimensions",
                            "dimensions must be positive",
                            head,
                        ),
                    ));
                }
            }

            let expected: usize = shape.iter().product();
            if expected != matrix.array.len() {
                return Err(ShellError::Generic(
                    nu_protocol::shell_error::generic::GenericError::new(
                        "Shape mismatch",
                        format!(
                            "cannot reshape {} elements into shape {:?} (would have {} elements)",
                            matrix.array.len(),
                            shape,
                            expected
                        ),
                        head,
                    ),
                ));
            }

            matrix
                .array
                .into_shape_with_order(ndarray::IxDyn(&shape))
                .map_err(|e| {
                    ShellError::Generic(nu_protocol::shell_error::generic::GenericError::new(
                        "Reshape error",
                        e.to_string(),
                        head,
                    ))
                })?
        };

        Ok(MatrixValue::new(result)
            .into_value(head)
            .into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                description: "Reshape a 1x6 matrix to 2x3",
                example: "[[1 2 3 4 5 6]] | into matrix | matrix reshape 2 3 | matrix into-nu | to nuon",
                result: Some(Value::test_string("[[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]")),
            },
            Example {
                description: "Flatten a 2x2 matrix to 1D",
                example: "matrix identity 2 | matrix reshape --flatten | matrix into-nu | to nuon",
                result: Some(Value::test_string("[[1.0, 0.0, 0.0, 1.0]]")),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(MatrixReshape)
    }
}
