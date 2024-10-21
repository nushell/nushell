use nu_engine::{command_prelude::*, get_eval_block_with_early_return};
use nu_protocol::{
    engine::{Call, Closure, Command, EngineState, Stack},
    DataSource, PipelineData, PipelineMetadata, ShellError, Signature, SyntaxShape, Type, Value,
};

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
        let metadata_record = build_metadata_record(input.metadata().as_ref(), call.head);

        if let Some(var_id) = block.signature.get_positional(0).and_then(|var| var.var_id) {
            callee_stack.add_var(var_id, metadata_record)
        }

        let eval = get_eval_block_with_early_return(engine_state);
        eval(engine_state, &mut callee_stack, block, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Access metadata and data from a stream together",
            example: r#"{foo: bar} | to json --raw | metadata access {|meta| {in: $in, meta: $meta}}"#,
            result: Some(Value::test_record(record! {
                "in" => Value::test_string(r#"{"foo":"bar"}"#),
                "meta" => Value::test_record(record! {
                    "content_type" => Value::test_string(r#"application/json"#)
                })
            })),
        }]
    }
}

fn build_metadata_record(metadata: Option<&PipelineMetadata>, head: Span) -> Value {
    let mut record = Record::new();

    if let Some(x) = metadata {
        match x {
            PipelineMetadata {
                data_source: DataSource::Ls,
                ..
            } => record.push("source", Value::string("ls", head)),
            PipelineMetadata {
                data_source: DataSource::HtmlThemes,
                ..
            } => record.push("source", Value::string("into html --list", head)),
            PipelineMetadata {
                data_source: DataSource::FilePath(path),
                ..
            } => record.push(
                "source",
                Value::string(path.to_string_lossy().to_string(), head),
            ),
            _ => {}
        }
        if let Some(ref content_type) = x.content_type {
            record.push("content_type", Value::string(content_type, head));
        }
    }

    Value::record(record, head)
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
