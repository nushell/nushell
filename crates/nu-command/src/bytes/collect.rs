use itertools::Itertools;
use nu_engine::command_prelude::*;

#[derive(Clone, Copy)]
pub struct BytesCollect;

impl Command for BytesCollect {
    fn name(&self) -> &str {
        "bytes collect"
    }

    fn signature(&self) -> Signature {
        Signature::build("bytes collect")
            .input_output_types(vec![(Type::List(Box::new(Type::Binary)), Type::Binary)])
            .optional(
                "separator",
                SyntaxShape::Binary,
                "Optional separator to use when creating binary.",
            )
            .category(Category::Bytes)
    }

    fn description(&self) -> &str {
        "Concatenate multiple binary into a single binary, with an optional separator between each."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["join", "concatenate"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let separator: Option<Vec<u8>> = call.opt(engine_state, stack, 0)?;

        let span = call.head;

        // input should be a list of binary data.
        let metadata = input.metadata();
        let iter = Itertools::intersperse(
            input.into_iter_strict(span)?.map(move |value| {
                // Everything is wrapped in Some in case there's a separator, so we can flatten
                Some(match value {
                    // Explicitly propagate errors instead of dropping them.
                    Value::Error { error, .. } => Err(*error),
                    Value::Binary { val, .. } => Ok(val),
                    other => Err(ShellError::OnlySupportsThisInputType {
                        exp_input_type: "binary".into(),
                        wrong_type: other.get_type().to_string(),
                        dst_span: span,
                        src_span: other.span(),
                    }),
                })
            }),
            Ok(separator).transpose(),
        )
        .flatten();

        let output = ByteStream::from_result_iter(
            iter,
            span,
            engine_state.signals().clone(),
            ByteStreamType::Binary,
        );

        Ok(PipelineData::byte_stream(output, metadata))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Create a byte array from input",
                example: "[0x[11] 0x[13 15]] | bytes collect",
                result: Some(Value::binary(vec![0x11, 0x13, 0x15], Span::test_data())),
            },
            Example {
                description: "Create a byte array from input with a separator",
                example: "[0x[11] 0x[33] 0x[44]] | bytes collect 0x[01]",
                result: Some(Value::binary(
                    vec![0x11, 0x01, 0x33, 0x01, 0x44],
                    Span::test_data(),
                )),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(BytesCollect {})
    }
}
