use crate::matrix::MatrixValue;
use crate::matrix::value::value_to_f64;
use ndarray::ArrayD;
use nu_engine::ClosureEvalOnce;
use nu_engine::command_prelude::*;
use nu_protocol::engine::Closure;

#[derive(Clone)]
pub struct MatrixMap;

impl Command for MatrixMap {
    fn name(&self) -> &str {
        "matrix map"
    }

    fn signature(&self) -> Signature {
        Signature::build("matrix map")
            .input_output_types(vec![(
                Type::Custom("matrix".into()),
                Type::Custom("matrix".into()),
            )])
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Number])),
                "The closure to apply to each element.",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Apply a closure to each element of a matrix and return a new matrix."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["each", "apply", "element"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let closure: Closure = call.req(engine_state, stack, 0)?;
        let matrix = MatrixValue::from_value(&input.into_value(head)?)?;

        let shape = matrix.array.shape().to_vec();
        let mut vals = Vec::with_capacity(matrix.array.len());
        for &val in matrix.array.iter() {
            let element = Value::float(val, head);
            let result = ClosureEvalOnce::new(engine_state, stack, closure.clone())
                .run_with_value(element)?
                .into_value(head)?;
            vals.push(value_to_f64(&result, head).map_err(|_| {
                ShellError::Generic(nu_protocol::shell_error::generic::GenericError::new(
                    "Invalid result",
                    "closure must return a number",
                    head,
                ))
            })?);
        }

        let array = ArrayD::from_shape_vec(shape, vals).map_err(|e| {
            ShellError::Generic(nu_protocol::shell_error::generic::GenericError::new(
                "Reshape error",
                e.to_string(),
                head,
            ))
        })?;

        Ok(MatrixValue::new(array)
            .into_value(head)
            .into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                description: "Double each element in a matrix",
                example: "[[1 2] [3 4]] | into matrix | matrix map {|e| $e * 2} | matrix into-nu | to nuon",
                result: Some(Value::test_string("[[2.0, 4.0], [6.0, 8.0]]")),
            },
            Example {
                description: "Add 10 to each element of an identity matrix",
                example: "matrix identity 2 | matrix map {|e| $e + 10} | matrix into-nu | to nuon",
                result: Some(Value::test_string("[[11.0, 10.0], [10.0, 11.0]]")),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(MatrixMap)
    }
}
