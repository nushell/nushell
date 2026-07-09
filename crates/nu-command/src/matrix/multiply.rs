use crate::matrix::MatrixValue;
use ndarray::ArrayD;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MatrixMultiply;

impl Command for MatrixMultiply {
    fn name(&self) -> &str {
        "matrix multiply"
    }

    fn signature(&self) -> Signature {
        Signature::build("matrix multiply")
            .input_output_types(vec![(
                Type::Custom("matrix".into()),
                Type::Custom("matrix".into()),
            )])
            .required(
                "other",
                SyntaxShape::Any,
                "The other matrix to multiply with.",
            )
            .switch("swap", "Swap the left and right operands", Some('s'))
            .switch(
                "multall",
                "Chain-multiply all supplied matrices: input @ other @ rest[0] @ ...",
                Some('a'),
            )
            .rest(
                "rest",
                SyntaxShape::Any,
                "Additional matrices for chained multiplication (requires --multall)",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Multiply two matrices using dot product."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["dot", "matmul", "product", "chain"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let swap = call.has_flag(engine_state, stack, "swap")?;
        let multall = call.has_flag(engine_state, stack, "multall")?;
        let other_val: Value = call.req(engine_state, stack, 0)?;

        let mut a = MatrixValue::from_value(&input.into_value(head)?)?;
        let mut b = MatrixValue::from_value(&other_val)?;

        if swap {
            std::mem::swap(&mut a, &mut b);
        }

        let rest: Vec<Value> = call.rest(engine_state, stack, 1)?;

        let mut result = multiply_arrays(a.array, b.array, head)?;

        if multall {
            for val in rest {
                let mat = MatrixValue::from_value(&val)?;
                result = multiply_arrays(result, mat.array, head)?;
            }
        }

        if result.ndim() == 0 {
            Ok(Value::float(result.first().copied().unwrap_or(0.0), head).into_pipeline_data())
        } else {
            Ok(MatrixValue::new(result)
                .into_value(head)
                .into_pipeline_data())
        }
    }

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                description: "Multiply two 2x2 matrices",
                example: "[[1 2] [3 4]] | into matrix | matrix multiply ([[1 0] [0 1]] | into matrix) | matrix into-nu | to nuon",
                result: Some(Value::test_string("[[1.0, 2.0], [3.0, 4.0]]")),
            },
            Example {
                description: "Multiply a matrix by its inverse gives identity",
                example: "matrix identity 2 | matrix multiply (matrix identity 2) | matrix into-nu | to nuon",
                result: Some(Value::test_string("[[1.0, 0.0], [0.0, 1.0]]")),
            },
            Example {
                description: "Swap operands with --swap",
                example: "[[1 2] [3 4]] | into matrix | matrix multiply --swap ([[0 1] [1 0]] | into matrix) | matrix into-nu | to nuon",
                result: Some(Value::test_string("[[3.0, 4.0], [1.0, 2.0]]")),
            },
            Example {
                description: "Chain-multiply three matrices with --multall",
                example: "matrix identity 2 | matrix multiply --multall ([[2 0] [0 2]] | into matrix) ([[3 0] [0 3]] | into matrix) | matrix into-nu | to nuon",
                result: Some(Value::test_string("[[6.0, 0.0], [0.0, 6.0]]")),
            },
        ]
    }
}

fn multiply_arrays(a: ArrayD<f64>, b: ArrayD<f64>, head: Span) -> Result<ArrayD<f64>, ShellError> {
    match (a.ndim(), b.ndim()) {
        (1, 1) => {
            if a.len() != b.len() {
                return Err(ShellError::Generic(
                    nu_protocol::shell_error::generic::GenericError::new(
                        "Shape mismatch",
                        format!(
                            "vectors must have the same length: {} vs {}",
                            a.len(),
                            b.len()
                        ),
                        head,
                    ),
                ));
            }
            let dot: f64 = ndarray::Zip::from(&a)
                .and(&b)
                .fold(0.0, |acc, &x, &y| acc + x * y);
            Ok(ArrayD::from_shape_vec(vec![], vec![dot]).map_err(|e| {
                ShellError::Generic(nu_protocol::shell_error::generic::GenericError::new(
                    "Shape error",
                    e.to_string(),
                    head,
                ))
            })?)
        }
        (2, 1) => {
            let a_view = a
                .view()
                .into_dimensionality::<ndarray::Ix2>()
                .map_err(|e| {
                    ShellError::Generic(nu_protocol::shell_error::generic::GenericError::new(
                        "Dimension error",
                        e.to_string(),
                        head,
                    ))
                })?;
            let b_view = b
                .view()
                .into_dimensionality::<ndarray::Ix1>()
                .map_err(|_| {
                    ShellError::Generic(nu_protocol::shell_error::generic::GenericError::new(
                        "Dimension error",
                        "expected a 1D vector",
                        head,
                    ))
                })?;
            if a_view.shape()[1] != b_view.shape()[0] {
                return Err(ShellError::Generic(
                    nu_protocol::shell_error::generic::GenericError::new(
                        "Shape mismatch",
                        format!(
                            "inner dimensions do not match: ({} x {}) dot {}",
                            a_view.shape()[0],
                            a_view.shape()[1],
                            b_view.shape()[0],
                        ),
                        head,
                    ),
                ));
            }
            let result = a_view.dot(&b_view);
            Ok(result.into_dyn())
        }
        (1, 2) => {
            let a_view = a
                .view()
                .into_dimensionality::<ndarray::Ix1>()
                .map_err(|_| {
                    ShellError::Generic(nu_protocol::shell_error::generic::GenericError::new(
                        "Dimension error",
                        "expected a 1D vector",
                        head,
                    ))
                })?;
            let b_view = b
                .view()
                .into_dimensionality::<ndarray::Ix2>()
                .map_err(|e| {
                    ShellError::Generic(nu_protocol::shell_error::generic::GenericError::new(
                        "Dimension error",
                        e.to_string(),
                        head,
                    ))
                })?;
            if a_view.shape()[0] != b_view.shape()[0] {
                return Err(ShellError::Generic(
                    nu_protocol::shell_error::generic::GenericError::new(
                        "Shape mismatch",
                        format!(
                            "inner dimensions do not match: {} dot {}",
                            a_view.shape()[0],
                            b_view.shape()[0],
                        ),
                        head,
                    ),
                ));
            }
            let result = a_view.dot(&b_view);
            Ok(result.into_dyn())
        }
        (2, 2) => {
            let a_view = a
                .view()
                .into_dimensionality::<ndarray::Ix2>()
                .map_err(|e| {
                    ShellError::Generic(nu_protocol::shell_error::generic::GenericError::new(
                        "Dimension error",
                        e.to_string(),
                        head,
                    ))
                })?;
            let b_view = b
                .view()
                .into_dimensionality::<ndarray::Ix2>()
                .map_err(|e| {
                    ShellError::Generic(nu_protocol::shell_error::generic::GenericError::new(
                        "Dimension error",
                        e.to_string(),
                        head,
                    ))
                })?;
            if a_view.shape()[1] != b_view.shape()[0] {
                return Err(ShellError::Generic(
                    nu_protocol::shell_error::generic::GenericError::new(
                        "Shape mismatch",
                        format!(
                            "inner dimensions do not match: ({} x {}) dot ({} x {})",
                            a_view.shape()[0],
                            a_view.shape()[1],
                            b_view.shape()[0],
                            b_view.shape()[1],
                        ),
                        head,
                    ),
                ));
            }
            let result = a_view.dot(&b_view);
            Ok(result.into_dyn())
        }
        _ => Err(ShellError::Generic(
            nu_protocol::shell_error::generic::GenericError::new(
                "Unsupported dimensions",
                format!(
                    "matrix multiply only supports 1D and 2D arrays, got shapes {:?} and {:?}",
                    a.shape(),
                    b.shape()
                ),
                head,
            ),
        )),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(MatrixMultiply)
    }

    #[test]
    fn test_incompatible_matrix_dimensions_error() {
        let head = Span::test_data();
        // (2x3) with (4x2) — inner dims 3 != 4
        let a = ArrayD::from_shape_vec(vec![2, 3], vec![1.0; 6]).unwrap();
        let b = ArrayD::from_shape_vec(vec![4, 2], vec![1.0; 8]).unwrap();
        let result = multiply_arrays(a, b, head);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("Shape mismatch"),
            "expected shape mismatch error, got: {msg}"
        );
    }

    #[test]
    fn test_incompatible_matrix_vector_dimensions_error() {
        let head = Span::test_data();
        // (2x3) with 4-element vector — inner dims 3 != 4
        let a = ArrayD::from_shape_vec(vec![2, 3], vec![1.0; 6]).unwrap();
        let b = ArrayD::from_shape_vec(vec![4], vec![1.0; 4]).unwrap();
        let result = multiply_arrays(a, b, head);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("Shape mismatch"),
            "expected shape mismatch error, got: {msg}"
        );
    }

    #[test]
    fn test_incompatible_vector_matrix_dimensions_error() {
        let head = Span::test_data();
        // 3-element vector with (4x2) — inner dims 3 != 4
        let a = ArrayD::from_shape_vec(vec![3], vec![1.0; 3]).unwrap();
        let b = ArrayD::from_shape_vec(vec![4, 2], vec![1.0; 8]).unwrap();
        let result = multiply_arrays(a, b, head);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("Shape mismatch"),
            "expected shape mismatch error, got: {msg}"
        );
    }
}
