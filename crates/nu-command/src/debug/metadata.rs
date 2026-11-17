use super::util::build_metadata_record;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct Metadata;

impl Command for Metadata {
    fn name(&self) -> &str {
        "metadata"
    }

    fn description(&self) -> &str {
        "Get the metadata for items in the stream."
    }

    fn extra_description(&self) -> &str {
        "With an expression, the result describes only that expression (its span) and pipeline input is ignored. Without one, the result is the input's pipeline metadata."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("metadata")
            .input_output_types(vec![(Type::Any, Type::record())])
            .allow_variants_without_examples(true)
            .optional(
                "expression",
                SyntaxShape::Any,
                "The expression you want metadata for. When given, pipeline input is ignored.",
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

        // An expression wins over the input: the first statement of a block receives the
        // block's input implicitly, so `each {|x| metadata $x }` gets both. The record then
        // describes only the expression; pipeline metadata comes from `metadata` alone.
        let record = match arg {
            Some(val) => record! { "span" => val.span().into_value(head) },
            None => build_metadata_record(&input, head),
        };
        Ok(Value::record(record, head).into_pipeline_data())
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
            Example {
                description: "Get the metadata of a closure parameter. The closure's input is ignored.",
                example: "[1] | each {|x| metadata $x } | columns",
                result: Some(Value::test_list(vec![Value::test_string("span")])),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(Metadata)
    }
}
