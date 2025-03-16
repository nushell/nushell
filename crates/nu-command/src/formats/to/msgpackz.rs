use std::io::Write;

use nu_engine::command_prelude::*;
use nu_protocol::shell_error::io::IoError;

use super::msgpack::write_value;

const BUFFER_SIZE: usize = 65536;
const DEFAULT_QUALITY: u32 = 3; // 1 can be very bad
const DEFAULT_WINDOW_SIZE: u32 = 20;

#[derive(Clone)]
pub struct ToMsgpackz;

impl Command for ToMsgpackz {
    fn name(&self) -> &str {
        "to msgpackz"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(Type::Any, Type::Binary)
            .named(
                "quality",
                SyntaxShape::Int,
                "Quality of brotli compression (default 3)",
                Some('q'),
            )
            .named(
                "window-size",
                SyntaxShape::Int,
                "Window size for brotli compression (default 20)",
                Some('w'),
            )
            .switch(
                "serialize",
                "serialize nushell types that cannot be deserialized",
                Some('s'),
            )
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Convert Nu values into brotli-compressed MessagePack."
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
        fn to_u32(n: Spanned<i64>) -> Result<Spanned<u32>, ShellError> {
            u32::try_from(n.item)
                .map_err(|err| ShellError::CantConvert {
                    to_type: "u32".into(),
                    from_type: "int".into(),
                    span: n.span,
                    help: Some(err.to_string()),
                })
                .map(|o| o.into_spanned(n.span))
        }

        let quality = call
            .get_flag(engine_state, stack, "quality")?
            .map(to_u32)
            .transpose()?;
        let window_size = call
            .get_flag(engine_state, stack, "window-size")?
            .map(to_u32)
            .transpose()?;
        let serialize_types = call.has_flag(engine_state, stack, "serialize")?;

        let value_span = input.span().unwrap_or(call.head);
        let value = input.into_value(value_span)?;
        let mut out_buf = vec![];
        let mut out = brotli::CompressorWriter::new(
            &mut out_buf,
            BUFFER_SIZE,
            quality.map(|q| q.item).unwrap_or(DEFAULT_QUALITY),
            window_size.map(|w| w.item).unwrap_or(DEFAULT_WINDOW_SIZE),
        );

        write_value(
            &mut out,
            &value,
            0,
            engine_state,
            call.head,
            serialize_types,
        )?;
        out.flush()
            .map_err(|err| IoError::new(err.kind(), call.head, None))?;
        drop(out);

        Ok(Value::binary(out_buf, call.head).into_pipeline_data())
    }
}
