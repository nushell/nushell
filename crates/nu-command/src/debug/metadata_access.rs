use nu_engine::{command_prelude::*, get_eval_block_with_early_return};
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
        let block = engine_state.get_block(closure.block_id);

        // `ClosureEvalOnce` is not used as it uses `Stack::captures_to_stack` rather than
        // `Stack::captures_to_stack_preserve_out_dest`. This command shouldn't collect streams
        let mut callee_stack = caller_stack.captures_to_stack_preserve_out_dest(closure.captures);
        let metadata_record = Value::record(build_metadata_record(&input, call.head), call.head);

        if let Some(var_id) = block.signature.get_positional(0).and_then(|var| var.var_id) {
            callee_stack.add_var(var_id, metadata_record)
        }

        let eval = get_eval_block_with_early_return(engine_state);
        eval(engine_state, &mut callee_stack, block, input).map(|p| p.body)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Access metadata and data from a stream together",
            example: r#"{foo: bar} | to json --raw | metadata access {|meta| {in: $in, content: $meta.content_type}}"#,
            result: Some(Value::test_record(record! {
                "in" => Value::test_string(r#"{"foo":"bar"}"#),
                "content" => Value::test_string(r#"application/json"#)
            })),
        }]
    }
}

#[cfg(test)]
mod test {
    use crate::ToJson;

    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples_with_commands;

        test_examples_with_commands(MetadataAccess {}, &[&ToJson])
    }
}
