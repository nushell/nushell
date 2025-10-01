use data_encoding::Encoding;

use nu_engine::command_prelude::*;

const EXTRA_USAGE: &str = r"The default alphabet is taken from RFC 4648, section 4.  A URL-safe version is available.

Note this command will collect stream input.";

fn get_encoding_from_flags(url: bool, nopad: bool) -> Encoding {
    match (url, nopad) {
        (false, false) => data_encoding::BASE64,
        (false, true) => data_encoding::BASE64_NOPAD,
        (true, false) => data_encoding::BASE64URL,
        (true, true) => data_encoding::BASE64URL_NOPAD,
    }
}

fn get_encoding(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<Encoding, ShellError> {
    let url = call.has_flag(engine_state, stack, "url")?;
    let nopad = call.has_flag(engine_state, stack, "nopad")?;

    Ok(get_encoding_from_flags(url, nopad))
}

fn get_encoding_const(working_set: &StateWorkingSet, call: &Call) -> Result<Encoding, ShellError> {
    let url = call.has_flag_const(working_set, "url")?;
    let nopad = call.has_flag_const(working_set, "nopad")?;

    Ok(get_encoding_from_flags(url, nopad))
}

#[derive(Clone)]
pub struct DecodeBase64;

impl Command for DecodeBase64 {
    fn name(&self) -> &str {
        "decode base64"
    }

    fn signature(&self) -> Signature {
        Signature::build("decode base64")
            .input_output_types(vec![(Type::String, Type::Binary)])
            .allow_variants_without_examples(true)
            .switch("url", "Decode the URL-safe Base64 version.", None)
            .switch("nopad", "Reject padding.", None)
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Decode a Base64 value."
    }

    fn extra_description(&self) -> &str {
        EXTRA_USAGE
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Decode a Base64 string",
                example: r#""U29tZSBEYXRh" | decode base64 | decode"#,
                result: None,
            },
            Example {
                description: "Decode arbitrary data",
                example: r#""/w==" | decode base64"#,
                result: Some(Value::test_binary(vec![0xFF])),
            },
            Example {
                description: "Decode a URL-safe Base64 string",
                example: r#""_w==" | decode base64 --url"#,
                result: Some(Value::test_binary(vec![0xFF])),
            },
        ]
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let encoding = get_encoding(engine_state, stack, call)?;
        super::decode(encoding, call.head, input)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let encoding = get_encoding_const(working_set, call)?;
        super::decode(encoding, call.head, input)
    }
}

#[derive(Clone)]
pub struct EncodeBase64;

impl Command for EncodeBase64 {
    fn name(&self) -> &str {
        "encode base64"
    }

    fn signature(&self) -> Signature {
        Signature::build("encode base64")
            .input_output_types(vec![
                (Type::String, Type::String),
                (Type::Binary, Type::String),
            ])
            .switch("url", "Use the URL-safe Base64 version.", None)
            .switch("nopad", "Don't pad the output.", None)
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Encode a string or binary value using Base64."
    }

    fn extra_description(&self) -> &str {
        EXTRA_USAGE
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Encode a string with Base64",
                example: r#""Alphabet from A to Z" | encode base64"#,
                result: Some(Value::test_string("QWxwaGFiZXQgZnJvbSBBIHRvIFo=")),
            },
            Example {
                description: "Encode arbitrary data",
                example: r#"0x[BE EE FF] | encode base64"#,
                result: Some(Value::test_string("vu7/")),
            },
            Example {
                description: "Use a URL-safe alphabet",
                example: r#"0x[BE EE FF] | encode base64 --url"#,
                result: Some(Value::test_string("vu7_")),
            },
        ]
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let encoding = get_encoding(engine_state, stack, call)?;
        super::encode(encoding, call.head, input)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let encoding = get_encoding_const(working_set, call)?;
        super::encode(encoding, call.head, input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples_decode() {
        crate::test_examples(DecodeBase64)
    }

    #[test]
    fn test_examples_encode() {
        crate::test_examples(EncodeBase64)
    }
}
