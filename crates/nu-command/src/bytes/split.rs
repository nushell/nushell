use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct BytesSplit;

impl Command for BytesSplit {
    fn name(&self) -> &str {
        "bytes split"
    }

    fn signature(&self) -> Signature {
        Signature::build("bytes split")
            .input_output_types(vec![(Type::Binary, Type::list(Type::Binary))])
            .required(
                "separator",
                SyntaxShape::OneOf(vec![SyntaxShape::Binary, SyntaxShape::String]),
                "Bytes or string that the input will be split on (must be non-empty).",
            )
            .category(Category::Bytes)
    }

    fn description(&self) -> &str {
        "Split input into multiple items using a separator."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let Spanned {
            item: separator,
            span,
        }: Spanned<Vec<u8>> = call.req(engine_state, stack, 0)?;

        if separator.is_empty() {
            return Err(ShellError::IncorrectValue {
                msg: "Separator can't be empty".into(),
                val_span: span,
                call_span: call.head,
            });
        }

        let (split_read, md) = match input {
            PipelineData::Value(Value::Binary { val, .. }, md) => (
                ByteStream::read_binary(val, head, engine_state.signals().clone()).split(separator),
                md,
            ),
            PipelineData::ByteStream(stream, md) => (stream.split(separator), md),
            input => {
                let span = input.span().unwrap_or(head);
                return Err(input.unsupported_input_error("bytes", span));
            }
        };
        if let Some(split) = split_read {
            Ok(split
                .map(move |part| match part {
                    Ok(val) => Value::binary(val, head),
                    Err(err) => Value::error(err, head),
                })
                .into_pipeline_data_with_metadata(head, engine_state.signals().clone(), md))
        } else {
            Ok(PipelineData::empty())
        }
    }
}
