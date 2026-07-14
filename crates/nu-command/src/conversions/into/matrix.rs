use crate::matrix::MatrixValue;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct IntoMatrix;

impl Command for IntoMatrix {
    fn name(&self) -> &str {
        "into matrix"
    }

    fn signature(&self) -> Signature {
        Signature::build("into matrix")
            .input_output_types(vec![
                (Type::table(), Type::Custom("matrix".into())),
                (
                    Type::List(Box::new(Type::List(Box::new(Type::Number)))),
                    Type::Custom("matrix".into()),
                ),
            ])
            .category(Category::Conversions)
    }

    fn description(&self) -> &str {
        "Convert a nushell table or list of lists into a matrix."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "array", "ndarray", "2d"]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let values: Vec<Value> = input.into_iter().collect();
        into_matrix(&values, head)
    }

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                description: "Convert a list of lists to a matrix",
                example: "[[1 2 3] [4 5 6]] | into matrix | matrix into-nu | to nuon",
                result: Some(Value::test_string("[[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]")),
            },
            Example {
                description: "Convert a list of records to a matrix",
                example: "[{a: 1 b: 2} {a: 3 b: 4}] | into matrix | matrix into-nu | to nuon",
                result: Some(Value::test_string("[[1.0, 2.0], [3.0, 4.0]]")),
            },
        ]
    }
}

fn into_matrix(values: &[Value], span: Span) -> Result<PipelineData, ShellError> {
    if values.is_empty() {
        let array = ndarray::ArrayD::from_shape_vec(vec![0, 0], vec![]).map_err(|e| {
            ShellError::Generic(nu_protocol::shell_error::generic::GenericError::new(
                "Matrix shape error",
                e.to_string(),
                span,
            ))
        })?;
        return Ok(MatrixValue::new(array)
            .into_value(span)
            .into_pipeline_data());
    }

    match &values[0] {
        Value::List { .. } => {
            let matrix = MatrixValue::from_list_of_lists(values, span)?;
            Ok(matrix.into_value(span).into_pipeline_data())
        }
        Value::Record { .. } => {
            let matrix = MatrixValue::from_list_of_records(values, span)?;
            Ok(matrix.into_value(span).into_pipeline_data())
        }
        Value::Custom { val, .. } if val.type_name() == "matrix" => {
            Ok(val.clone_value(span).into_pipeline_data())
        }
        _ => Err(ShellError::Generic(
            nu_protocol::shell_error::generic::GenericError::new(
                "Invalid input",
                "expected a list of lists, a list of records, or a matrix",
                span,
            ),
        )),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(IntoMatrix)
    }
}
