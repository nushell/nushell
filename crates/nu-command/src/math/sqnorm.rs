use crate::math::dot::compute_dot_product;
use crate::math::utils::run_with_function;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math sqnorm"
    }

    fn signature(&self) -> Signature {
        Signature::build("math sqnorm")
            .input_output_types(vec![(Type::List(Box::new(Type::Number)), Type::Number)])
            .allow_variants_without_examples(true)
            .category(Category::Math)
    }

    fn description(&self) -> &str {
        "Returns the squared norm of two lists of numbers, interpreting both as vectors."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["vector", "squared norm"]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;

        // This doesn't match explicit nulls
        if matches!(input, PipelineData::Empty) {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }

        run_with_function(call, input, |vector_lhs, pipeline_span, command_span| {
            compute_squared_norm(vector_lhs, pipeline_span, command_span)
        })
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Calculate the squared norm as vectors",
            example: "[1, 2, 3] | math sqnorm",
            result: Some(Value::test_int(-4)),
        }]
    }
}

pub fn compute_squared_norm(
    vector: &[Value],
    argument_span: Span,
    command_span: Span,
) -> Result<Value, ShellError> {
    compute_dot_product(vector, vector, argument_span, command_span)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
