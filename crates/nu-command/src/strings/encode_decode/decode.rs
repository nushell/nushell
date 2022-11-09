use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Spanned,
    SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Decode;

impl Command for Decode {
    fn name(&self) -> &str {
        "decode"
    }

    fn usage(&self) -> &str {
        "Decode bytes as a string."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["text", "encoding", "decoding"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("decode")
            .input_output_types(vec![(Type::Binary, Type::String)])
            .required("encoding", SyntaxShape::String, "the text encoding to use")
            .category(Category::Strings)
    }

    fn extra_usage(&self) -> &str {
        r#"Multiple encodings are supported, here is an example of a few:
big5, euc-jp, euc-kr, gbk, iso-8859-1, utf-16, cp1252, latin5

For a more complete list of encodings please refer to the encoding_rs
documentation link at https://docs.rs/encoding_rs/0.8.28/encoding_rs/#statics"#
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Decode the output of an external command",
                example: "^cat myfile.q | decode utf-8",
                result: None,
            },
            Example {
                description: "Decode an UTF-16 string into nushell UTF-8 string",
                example: r#"0x[00 53 00 6F 00 6D 00 65 00 20 00 44 00 61 00 74 00 61] | decode utf-16be"#,
                result: Some(Value::String {
                    val: "Some Data".to_owned(),
                    span: Span::test_data(),
                }),
            },
        ]
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
                let bytes: Vec<u8> = stream.into_bytes()?.item;
                super::encoding::decode(head, encoding, &bytes).map(|val| val.into_pipeline_data())
            }
            PipelineData::Value(Value::Binary { val: bytes, .. }, ..) => {
                super::encoding::decode(head, encoding, &bytes).map(|val| val.into_pipeline_data())
            }
            _ => Err(ShellError::UnsupportedInput(
                "non-binary input".into(),
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
        crate::test_examples(Decode)
    }
}
