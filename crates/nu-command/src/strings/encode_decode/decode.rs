use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct Decode;

impl Command for Decode {
    fn name(&self) -> &str {
        "decode"
    }

    fn usage(&self) -> &str {
        "Decode bytes into a string."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["text", "encoding", "decoding"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("decode")
            .input_output_types(vec![(Type::Binary, Type::String)])
            .optional("encoding", SyntaxShape::String, "The text encoding to use.")
            .category(Category::Strings)
    }

    fn extra_usage(&self) -> &str {
        r#"Multiple encodings are supported; here are a few:
big5, euc-jp, euc-kr, gbk, iso-8859-1, utf-16, cp1252, latin5

For a more complete list of encodings please refer to the encoding_rs
documentation link at https://docs.rs/encoding_rs/latest/encoding_rs/#statics"#
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
                result: Some(Value::string("Some Data".to_owned(), Span::test_data())),
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
        let encoding: Option<Spanned<String>> = call.opt(engine_state, stack, 0)?;

        match input {
            PipelineData::ExternalStream { stdout: None, .. } => Ok(PipelineData::empty()),
            PipelineData::ExternalStream {
                stdout: Some(stream),
                span: input_span,
                ..
            } => {
                let bytes: Vec<u8> = stream.into_bytes()?.item;
                match encoding {
                    Some(encoding_name) => super::encoding::decode(head, encoding_name, &bytes),
                    None => super::encoding::detect_encoding_name(head, input_span, &bytes)
                        .map(|encoding| encoding.decode(&bytes).0.into_owned())
                        .map(|s| Value::string(s, head)),
                }
                .map(|val| val.into_pipeline_data())
            }
            PipelineData::Value(v, ..) => {
                let input_span = v.span();
                match v {
                    Value::Binary { val: bytes, .. } => match encoding {
                        Some(encoding_name) => super::encoding::decode(head, encoding_name, &bytes),
                        None => super::encoding::detect_encoding_name(head, input_span, &bytes)
                            .map(|encoding| encoding.decode(&bytes).0.into_owned())
                            .map(|s| Value::string(s, head)),
                    }
                    .map(|val| val.into_pipeline_data()),
                    Value::Error { error, .. } => Err(*error),
                    _ => Err(ShellError::OnlySupportsThisInputType {
                        exp_input_type: "binary".into(),
                        wrong_type: v.get_type().to_string(),
                        dst_span: head,
                        src_span: v.span(),
                    }),
                }
            }
            // This should be more precise, but due to difficulties in getting spans
            // from PipelineData::ListData, this is as it is.
            _ => Err(ShellError::UnsupportedInput {
                msg: "non-binary input".into(),
                input: "value originates from here".into(),
                msg_span: head,
                input_span: input.span().unwrap_or(head),
            }),
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
