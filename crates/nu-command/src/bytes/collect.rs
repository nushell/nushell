use itertools::Itertools;
use nu_engine::command_prelude::*;
use nu_protocol::RawStream;

#[derive(Clone, Copy)]
pub struct BytesCollect;

impl Command for BytesCollect {
    fn name(&self) -> &str {
        "bytes collect"
    }

    fn signature(&self) -> Signature {
        Signature::build("bytes collect")
            .input_output_types(vec![(Type::List(Box::new(Type::Binary)), Type::Binary)])
            .switch(
                "stream",
                "Output the result as a raw stream, instead of collecting to a binary.",
                Some('s'),
            )
            .optional(
                "separator",
                SyntaxShape::Binary,
                "Optional separator to use when creating binary.",
            )
            .category(Category::Bytes)
    }

    fn usage(&self) -> &str {
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
        let should_stream = call.has_flag(engine_state, stack, "stream")?;
        let separator: Option<Vec<u8>> = call.opt(engine_state, stack, 0)?;

        let span = call.head;
        let metadata = input.metadata();

        // Create an iterator that contains individual chunks, interspersing the separator if it
        // was specified.
        //
        // This iterator doesn't borrow anything, so we can also use it to construct the
        // `RawStream`.
        let iter = Itertools::intersperse(
            input.into_iter().map(move |value| {
                // This is wrapped in Some so that we can intersperse an optional separator and then
                // flatten it without that
                Some(match value {
                    Value::Binary { val, .. } => Ok(val),
                    // Propagate errors
                    Value::Error { error, .. } => Err(*error),
                    // We only accept binary data
                    other => Err(ShellError::OnlySupportsThisInputType {
                        exp_input_type: "binary".into(),
                        wrong_type: other.get_type().to_string(),
                        dst_span: span,
                        src_span: other.span(),
                    }),
                })
            }),
            separator.map(Ok),
        )
        .flatten();

        if should_stream {
            Ok(PipelineData::ExternalStream {
                stdout: Some(RawStream::new(
                    Box::new(iter),
                    engine_state.ctrlc.clone(),
                    span,
                    None,
                )),
                stderr: None,
                exit_code: None,
                span,
                metadata,
                trim_end_newline: false,
            })
        } else {
            let mut binary = Vec::new();
            for chunk in iter {
                binary.extend_from_slice(&chunk?);
            }
            Ok(Value::binary(binary, span).into_pipeline_data())
        }
    }

    fn examples(&self) -> Vec<Example> {
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
            // TODO: replace this example with something that doesn't depend on endianness...
            // but our options for creating binary data algorithmically for the moment are a bit
            // limited
            Example {
                description: "Create a byte stream from a stream of chunks",
                example: "0x00..0x40..0x100 | each { into binary } | bytes collect --stream",
                result: Some(Value::test_binary(
                    [0x00, 0x40, 0x80, 0xC0, 0x100]
                        .into_iter()
                        .flat_map(i64::to_ne_bytes)
                        .collect::<Vec<u8>>(),
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
