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
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Multiply two matrices using dot product."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["dot", "matmul", "product"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let other_val: Value = call.req(engine_state, stack, 0)?;
        let a = MatrixValue::from_value(&input.into_value(head)?)?;
        let b = MatrixValue::from_value(&other_val)?;

        match (a.array.ndim(), b.array.ndim()) {
            (1, 1) => {
                if a.array.len() != b.array.len() {
                    return Err(ShellError::Generic(
                        nu_protocol::shell_error::generic::GenericError::new(
                            "Shape mismatch",
                            format!(
                                "vectors must have the same length: {} vs {}",
                                a.array.len(),
                                b.array.len()
                            ),
                            head,
                        ),
                    ));
                }
                let dot: f64 = ndarray::Zip::from(&a.array)
                    .and(&b.array)
                    .fold(0.0, |acc, &x, &y| acc + x * y);
                Ok(Value::float(dot, head).into_pipeline_data())
            }
            (2, 1) => {
                let a_view = a
                    .array
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
                    .array
                    .view()
                    .into_dimensionality::<ndarray::Ix1>()
                    .map_err(|_| {
                        ShellError::Generic(nu_protocol::shell_error::generic::GenericError::new(
                            "Dimension error",
                            "expected 1D vector",
                            head,
                        ))
                    })?;
                let result = a_view.dot(&b_view);
                let dyn_result: ArrayD<f64> = result.into_dyn();
                Ok(MatrixValue::new(dyn_result)
                    .into_value(head)
                    .into_pipeline_data())
            }
            (1, 2) => {
                let a_view = a
                    .array
                    .view()
                    .into_dimensionality::<ndarray::Ix1>()
                    .map_err(|_| {
                        ShellError::Generic(nu_protocol::shell_error::generic::GenericError::new(
                            "Dimension error",
                            "expected 1D vector",
                            head,
                        ))
                    })?;
                let b_view = b
                    .array
                    .view()
                    .into_dimensionality::<ndarray::Ix2>()
                    .map_err(|e| {
                        ShellError::Generic(nu_protocol::shell_error::generic::GenericError::new(
                            "Dimension error",
                            e.to_string(),
                            head,
                        ))
                    })?;
                let result = a_view.dot(&b_view);
                let dyn_result: ArrayD<f64> = result.into_dyn();
                Ok(MatrixValue::new(dyn_result)
                    .into_value(head)
                    .into_pipeline_data())
            }
            (2, 2) => {
                let a_view = a
                    .array
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
                    .array
                    .view()
                    .into_dimensionality::<ndarray::Ix2>()
                    .map_err(|e| {
                        ShellError::Generic(nu_protocol::shell_error::generic::GenericError::new(
                            "Dimension error",
                            e.to_string(),
                            head,
                        ))
                    })?;
                let result = a_view.dot(&b_view);
                let dyn_result: ArrayD<f64> = result.into_dyn();
                Ok(MatrixValue::new(dyn_result)
                    .into_value(head)
                    .into_pipeline_data())
            }
            _ => Err(ShellError::Generic(
                nu_protocol::shell_error::generic::GenericError::new(
                    "Unsupported dimensions",
                    format!(
                        "matrix multiply only supports 1D and 2D arrays, got shapes {:?} and {:?}",
                        a.array.shape(),
                        b.array.shape()
                    ),
                    head,
                ),
            )),
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
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(MatrixMultiply)
    }
}
