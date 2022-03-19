use encoding_rs::Encoding;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Spanned, SyntaxShape,
    Value,
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

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("decode")
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
        vec![Example {
            description: "Decode the output of an external command",
            example: "cat myfile.q | decode utf-8",
            result: None,
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
                let bytes: Vec<u8> = stream.into_bytes()?.item;

                let encoding = match Encoding::for_label(encoding.item.as_bytes()) {
                    None => Err(ShellError::SpannedLabeledError(
                        format!(
                            r#"{} is not a valid encoding, refer to https://docs.rs/encoding_rs/0.8.23/encoding_rs/#statics for a valid list of encodings"#,
                            encoding.item
                        ),
                        "invalid encoding".into(),
                        encoding.span,
                    )),
                    Some(encoding) => Ok(encoding),
                }?;

                let result = encoding.decode(&bytes);

                Ok(Value::String {
                    val: result.0.to_string(),
                    span: head,
                }
                .into_pipeline_data())
            }
            PipelineData::Value(Value::Binary { val: bytes, .. }, ..) => {
                let encoding = match Encoding::for_label(encoding.item.as_bytes()) {
                    None => Err(ShellError::SpannedLabeledError(
                        format!(
                            r#"{} is not a valid encoding, refer to https://docs.rs/encoding_rs/0.8.23/encoding_rs/#statics for a valid list of encodings"#,
                            encoding.item
                        ),
                        "invalid encoding".into(),
                        encoding.span,
                    )),
                    Some(encoding) => Ok(encoding),
                }?;

                let result = encoding.decode(&bytes);

                Ok(Value::String {
                    val: result.0.to_string(),
                    span: head,
                }
                .into_pipeline_data())
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
