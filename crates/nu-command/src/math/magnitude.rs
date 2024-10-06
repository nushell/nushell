use crate::math::sqnorm::compute_squared_norm;
use crate::math::utils::run_with_function;
use nu_engine::command_prelude::*;
use nu_protocol::IntoValue;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math magnitude"
    }

    fn signature(&self) -> Signature {
        Signature::build("math magnitude")
            .input_output_types(vec![(Type::List(Box::new(Type::Number)), Type::Number)])
            .allow_variants_without_examples(true)
            .category(Category::Math)
    }

    fn description(&self) -> &str {
        "Returns the magnitude of a vector, with elements as present in the pipeline."
    }

    fn extra_description(&self) -> &str {
        "This is equivalent to `math sqnorm | math sqrt`."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["vector", "length", "measure"]
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        operate(call, input)
    }

    fn run_const(
        &self,
        _working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        operate(call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Calculate the squared norm as vectors",
            example: "[1, 2, 3] | math sqnorm",
            result: Some(Value::test_int(-4)),
        }]
    }
}

fn operate(call: &Call, input: PipelineData) -> Result<PipelineData, ShellError> {
    let head = call.head;

    // This doesn't match explicit nulls
    if matches!(input, PipelineData::Empty) {
        return Err(ShellError::PipelineEmpty { dst_span: head });
    }

    run_with_function(call, input, |vector_lhs, pipeline_span, command_span| {
        compute_magnitude(vector_lhs, pipeline_span, command_span)
    })
}

pub fn compute_magnitude(
    vector: &[Value],
    argument_span: Span,
    command_span: Span,
) -> Result<Value, ShellError> {
    let squared_norm = compute_squared_norm(vector, argument_span, command_span)?;

    squared_norm
        .coerce_float()
        .map(|float| float.sqrt().into_value(command_span))
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
