use nu_engine::{ClosureEvalOnce, command_prelude::*};
use nu_protocol::{
    PipelineData, ShellError, Signature, SyntaxShape, Type, Value,
    engine::{Call, Closure, Command, EngineState, Stack},
};

use super::util::build_metadata_record;

#[derive(Clone)]
pub struct MetadataAccess;

impl Command for MetadataAccess {
    fn name(&self) -> &str {
        "metadata access"
    }

    fn description(&self) -> &str {
        "Access the metadata for the input stream within a closure."
    }

    fn signature(&self) -> Signature {
        Signature::build("metadata access")
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Record(vec![])])),
                "The closure to run with metadata access.",
            )
            .input_output_types(vec![(Type::Any, Type::Any)])
            .category(Category::Debug)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        caller_stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let closure: Closure = call.req(engine_state, caller_stack, 0)?;
        let metadata_record = Value::record(build_metadata_record(&input, call.head), call.head);

        ClosureEvalOnce::new_env_preserve_out_dest(engine_state, caller_stack, closure)
            .add_arg(metadata_record)?
            .run_with_input(input)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Access metadata and data from a stream together.",
            example: "{foo: bar} | to json --raw | metadata access {|meta| {in: $in, content: $meta.content_type}}",
            result: Some(Value::test_record(record! {
                "in" => Value::test_string(r#"{"foo":"bar"}"#),
                "content" => Value::test_string("application/json")
            })),
        }]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(MetadataAccess)
    }
}
