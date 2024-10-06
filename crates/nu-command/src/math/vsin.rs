use crate::math::utils::run_with_function;
use crate::math::vcos::compute_vcos;
use itertools::Itertools;
use nu_engine::command_prelude::*;
use nu_protocol::IntoValue;

struct Arguments {
    vector_rhs: Vec<f64>,
}

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math vsin"
    }

    fn signature(&self) -> Signature {
        Signature::build("math vsin")
            .input_output_types(vec![(Type::List(Box::new(Type::Number)), Type::Number)])
            .allow_variants_without_examples(true)
            .required(
                "second_vector",
                SyntaxShape::List(Box::new(SyntaxShape::Number)),
                "The second vector to compare to the vector in the pipeline.",
            )
            .category(Category::Math)
    }

    fn description(&self) -> &str {
        "Returns the sine of the angle between vectors, represented as lists."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["vector", "sine", "angle"]
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        operate(
            call,
            input,
            Arguments {
                vector_rhs: call.req::<Vec<f64>>(engine_state, stack, 0)?,
            },
        )
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        operate(
            call,
            input,
            Arguments {
                vector_rhs: call.req_const::<Vec<f64>>(working_set, 0)?,
            },
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Calculate the sine angle between two vectors, represented by lists",
            example: "[1, 2, 3] | math vsin [3, 4, -5]",
            result: Some(Value::test_int(-4)),
        }]
    }
}

fn operate(call: &Call, input: PipelineData, args: Arguments) -> Result<PipelineData, ShellError> {
    let head = call.head;

    // This doesn't match explicit nulls
    if matches!(input, PipelineData::Empty) {
        return Err(ShellError::PipelineEmpty { dst_span: head });
    }

    let vector_rhs = args
        .vector_rhs
        .iter()
        .map(|float| float.into_value(head))
        .collect_vec();
    let vector_rhs_span = call.arguments_span();

    run_with_function(call, input, |vector_lhs, pipeline_span, command_span| {
        compute_vsin(
            vector_lhs,
            vector_rhs.as_slice(),
            pipeline_span,
            vector_rhs_span,
            command_span,
        )
    })
}

pub fn compute_vsin(
    vector_lhs: &[Value],
    vector_rhs: &[Value],
    pipeline_span: Span,
    argument_span: Span,
    command_span: Span,
) -> Result<Value, ShellError> {
    if vector_lhs.len() != vector_rhs.len() {
        return Err(ShellError::IncorrectValue {
            msg: format!("Only equal-length vectors are supported.\nThe pipeline contained {} elements, this list contained {}.", vector_lhs.len(), vector_rhs.len()),
            val_span: argument_span,
            call_span: command_span,
        });
    }

    let cosine = compute_vcos(
        vector_lhs,
        vector_rhs,
        pipeline_span,
        argument_span,
        command_span,
    )?;
    let cosine_squared = cosine.mul(command_span, &cosine, command_span)?;
    let output_squared =
        Value::int(1, command_span).sub(command_span, &cosine_squared, command_span)?;

    output_squared
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
