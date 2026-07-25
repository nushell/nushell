use crate::matrix::MatrixValue;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MatrixMean;

impl Command for MatrixMean {
    fn name(&self) -> &str {
        "matrix mean"
    }

    fn signature(&self) -> Signature {
        Signature::build("matrix mean")
            .input_output_types(vec![(Type::Custom("matrix".into()), Type::Float)])
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Compute the mean of all elements in a matrix."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["average", "avg"]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let matrix = MatrixValue::from_value(&input.into_value(head)?)?;

        let count = matrix.array.len() as f64;
        if count == 0.0 {
            return Err(ShellError::Generic(
                nu_protocol::shell_error::generic::GenericError::new(
                    "Empty matrix",
                    "cannot compute mean of an empty matrix",
                    head,
                ),
            ));
        }

        let total: f64 = matrix.array.sum();
        let mean = total / count;

        Ok(Value::float(mean, head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                description: "Compute the mean of a 2x2 matrix",
                example: "[[1 2] [3 4]] | into matrix | matrix mean",
                result: Some(Value::test_float(2.5)),
            },
            Example {
                description: "Compute the mean of an identity matrix",
                example: "matrix identity 2 | matrix mean",
                result: Some(Value::test_float(0.5)),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(MatrixMean)
    }
}
