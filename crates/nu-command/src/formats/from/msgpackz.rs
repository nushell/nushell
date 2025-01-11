use std::io::Cursor;

use nu_engine::command_prelude::*;

use super::msgpack::{read_msgpack, Opts};

const BUFFER_SIZE: usize = 65536;

#[derive(Clone)]
pub struct FromMsgpackz;

impl Command for FromMsgpackz {
    fn name(&self) -> &str {
        "from msgpackz"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(Type::Binary, Type::Any)
            .switch("objects", "Read multiple objects from input", None)
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Convert brotli-compressed MessagePack data into Nu values."
    }

    fn extra_description(&self) -> &str {
        "This is the format used by the plugin registry file ($nu.plugin-path)."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = input.span().unwrap_or(call.head);
        let objects = call.has_flag(engine_state, stack, "objects")?;
        let opts = Opts {
            span,
            objects,
            signals: engine_state.signals().clone(),
        };
        let metadata = input.metadata().map(|md| md.with_content_type(None));
        let out = match input {
            // Deserialize from a byte buffer
            PipelineData::Value(Value::Binary { val: bytes, .. }, _) => {
                let reader = brotli::Decompressor::new(Cursor::new(bytes), BUFFER_SIZE);
                read_msgpack(reader, opts)
            }
            // Deserialize from a raw stream directly without having to collect it
            PipelineData::ByteStream(stream, ..) => {
                let span = stream.span();
                if let Some(reader) = stream.reader() {
                    let reader = brotli::Decompressor::new(reader, BUFFER_SIZE);
                    read_msgpack(reader, opts)
                } else {
                    Err(ShellError::PipelineMismatch {
                        exp_input_type: "binary or byte stream".into(),
                        dst_span: call.head,
                        src_span: span,
                    })
                }
            }
            _ => Err(ShellError::PipelineMismatch {
                exp_input_type: "binary or byte stream".into(),
                dst_span: call.head,
                src_span: span,
            }),
        };
        out.map(|pd| pd.set_metadata(metadata))
    }
}
