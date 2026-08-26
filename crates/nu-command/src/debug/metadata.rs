use super::util::{build_metadata_record, extend_record_with_metadata};
use nu_engine::command_prelude::*;
use nu_protocol::PipelineMetadata;

#[derive(Clone)]
pub struct Metadata;

impl Command for Metadata {
    fn name(&self) -> &str {
        "metadata"
    }

    fn description(&self) -> &str {
        "Get the metadata for items in the stream."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("metadata")
            .input_output_types(vec![(Type::Any, Type::record())])
            .allow_variants_without_examples(true)
            .optional(
                "expression",
                SyntaxShape::Any,
                "The expression you want metadata for.",
            )
            .category(Category::Debug)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        // Do not use `call.opt`: it filters out `Value::Nothing`, so `metadata $null` (and
        // optional/rest bindings that are nothing) would be treated as "no positional" and
        // return pipeline metadata without a `span` field. Callers like `assert equal $x null`
        // always evaluate `(metadata $right).span` when building error labels.
        let arg = if call.has_positional_args(stack, 0) {
            Some(call.req::<Value>(engine_state, stack, 0)?)
        } else {
            None
        };

        if !matches!(input, PipelineData::Empty)
            && let Some(ref arg_val) = arg
        {
            return Err(ShellError::IncompatibleParameters {
                left_message: "pipeline input was provided".into(),
                left_span: head,
                right_message: "but a positional metadata expression was also given".into(),
                right_span: arg_val.span(),
            });
        }

        match arg {
            Some(val) => Ok(
                build_metadata_record_value(&val, input.metadata_ref(), head).into_pipeline_data(),
            ),
            None => {
                Ok(Value::record(build_metadata_record(&input, head), head).into_pipeline_data())
            }
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Get the metadata of a variable.",
                example: "let a = 42; metadata $a",
                result: None,
            },
            Example {
                description: "Get the metadata of the input.",
                example: "ls | metadata",
                result: None,
            },
        ]
    }
}

fn build_metadata_record_value(
    arg: &Value,
    metadata: Option<&PipelineMetadata>,
    head: Span,
) -> Value {
    let mut record = Record::new();
    record.push("span", arg.span().into_value(head));
    Value::record(extend_record_with_metadata(record, metadata, head), head)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(Metadata)
    }
}
