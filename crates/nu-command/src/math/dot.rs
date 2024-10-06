use crate::math::reducers::sum;
use crate::math::utils::run_with_function;
use itertools::Itertools;
use nu_engine::command_prelude::*;
use nu_protocol::IntoValue;

struct Arguments {
    vector_rhs: Vec<f64>
}

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math dot"
    }

    fn signature(&self) -> Signature {
        Signature::build("math dot")
            .input_output_types(vec![(Type::List(Box::new(Type::Number)), Type::Number)])
            .allow_variants_without_examples(true)
            .required(
                "second_vector",
                SyntaxShape::List(Box::new(SyntaxShape::Number)),
                "The second vector to use in determining the dot product.",
            )
            .category(Category::Math)
    }

    fn description(&self) -> &str {
        "Returns the dot product of two lists of numbers, interpreting both as vectors."
    }

    fn extra_description(&self) -> &str {
        "This is equivalent to a pairwise multiplication of both lists, followed by a summation."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["vector", "dot product"]
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
        run(call, input, Arguments {
            vector_rhs: call.req::<Vec<f64>>(engine_state, stack, 0)?
        })
    }

    fn run_const(&self, working_set: &StateWorkingSet, call: &Call, input: PipelineData) -> Result<PipelineData, ShellError> {
        run(call, input, Arguments {
            vector_rhs: call.req_const::<Vec<f64>>(working_set, 0)?
        })
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Apply the dot product to two lists of numbers, interpreted as vectors",
            example: "[1, 2, 3] | math dot [3, 4, -5]",
            result: Some(Value::test_int(-4)),
        }]
    }
}

fn run(call: &Call, input: PipelineData, args: Arguments) -> Result<PipelineData, ShellError> {
    let head = call.head;

    // This doesn't match explicit nulls
    if matches!(input, PipelineData::Empty) {
        return Err(ShellError::PipelineEmpty { dst_span: head });
    }

    let vector_rhs = args.vector_rhs
        .iter()
        .map(|float| float.into_value(head))
        .collect_vec();
    let vector_rhs_span = call.arguments_span();

    run_with_function(call, input, |vector_lhs, _pipeline_span, command_span| {
        compute_dot_product(
            vector_lhs,
            vector_rhs.as_slice(),
            vector_rhs_span,
            command_span,
        )
    })
}

pub fn compute_dot_product(
    vector_lhs: &[Value],
    vector_rhs: &[Value],
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

    let vector_element_pairs = vector_lhs.iter().zip(vector_rhs);
    let element_products: Vec<Value> = vector_element_pairs
        .map(|(pipeline_value, arg_value)| {
            pipeline_value
                .mul(argument_span, arg_value, command_span)
                .unwrap_or(Value::float(0f64, command_span))
        })
        .collect_vec();

    sum(element_products, argument_span, command_span)
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
