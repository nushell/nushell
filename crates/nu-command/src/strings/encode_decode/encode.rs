use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Spanned,
    SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Encode;

impl Command for Encode {
    fn name(&self) -> &str {
        "encode"
    }

    fn usage(&self) -> &str {
        "Encode an UTF-8 string into other kind of representations."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["text", "encoding", "decoding"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("encode")
            .input_output_types(vec![(Type::String, Type::Binary)])
            .required("encoding", SyntaxShape::String, "the text encoding to use")
            .category(Category::Strings)
    }

    fn extra_usage(&self) -> &str {
        r#"Multiple encodings are supported, here is an example of a few:
big5, euc-jp, euc-kr, gbk, iso-8859-1, cp1252, latin5

Note that since the Encoding Standard doesn't specify encoders for utf-16le and utf-16be, these are not yet supported.

For a more complete list of encodings please refer to the encoding_rs
documentation link at https://docs.rs/encoding_rs/0.8.28/encoding_rs/#statics"#
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Encode an UTF-8 string into Shift-JIS",
            example: r#""負けると知って戦うのが、遥かに美しいのだ" | encode shift-jis"#,
            result: Some(Value::Binary {
                val: vec![
                    0x95, 0x89, 0x82, 0xaf, 0x82, 0xe9, 0x82, 0xc6, 0x92, 0x6d, 0x82, 0xc1, 0x82,
                    0xc4, 0x90, 0xed, 0x82, 0xa4, 0x82, 0xcc, 0x82, 0xaa, 0x81, 0x41, 0x97, 0x79,
                    0x82, 0xa9, 0x82, 0xc9, 0x94, 0xfc, 0x82, 0xb5, 0x82, 0xa2, 0x82, 0xcc, 0x82,
                    0xbe,
                ],
                span: Span::test_data(),
            }),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let encoding: Spanned<String> = call.req(engine_state, stack, 0)?;

        match input {
            PipelineData::ExternalStream { stdout: None, .. } => Ok(PipelineData::new(call.head)),
            PipelineData::ExternalStream {
                stdout: Some(stream),
                ..
            } => {
                let s = stream.into_string()?.item;
                super::encoding::encode(head, encoding, &s).map(|val| val.into_pipeline_data())
            }
            PipelineData::Value(Value::String { val: s, .. }, ..) => {
                super::encoding::encode(head, encoding, &s).map(|val| val.into_pipeline_data())
            }
            _ => Err(ShellError::UnsupportedInput(
                "non-string input".into(),
                head,
            )),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        crate::test_examples(Encode)
    }
}
