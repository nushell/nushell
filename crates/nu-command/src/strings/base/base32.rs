use data_encoding::Encoding;

use nu_engine::command_prelude::*;

const EXTRA_USAGE: &str = r"The default alphabet is taken from RFC 4648, section 6.

Note this command will collect stream input.";

#[derive(Clone)]
pub struct DecodeBase32;

impl Command for DecodeBase32 {
    fn name(&self) -> &str {
        "decode base32"
    }

    fn signature(&self) -> Signature {
        Signature::build("decode base32")
            .input_output_types(vec![(Type::String, Type::Binary)])
            .allow_variants_without_examples(true)
            .switch("nopad", "Do not pad the output.", None)
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Decode a Base32 value."
    }

    fn extra_description(&self) -> &str {
        EXTRA_USAGE
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Decode arbitrary binary data",
                example: r#""AEBAGBAF" | decode base32"#,
                result: Some(Value::test_binary(vec![1, 2, 3, 4, 5])),
            },
            Example {
                description: "Decode an encoded string",
                example: r#""NBUQ====" | decode base32 | decode"#,
                result: None,
            },
            Example {
                description: "Parse a string without padding",
                example: r#""NBUQ" | decode base32 --nopad"#,
                result: Some(Value::test_binary(vec![0x68, 0x69])),
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
        let encoding = if call.has_flag(engine_state, stack, "nopad")? {
            data_encoding::BASE32_NOPAD
        } else {
            data_encoding::BASE32
        };
        super::decode(encoding, call.span(), input)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let encoding = if call.has_flag_const(working_set, "nopad")? {
            data_encoding::BASE32_NOPAD
        } else {
            data_encoding::BASE32
        };
        super::decode(encoding, call.span(), input)
    }
}

#[derive(Clone)]
pub struct EncodeBase32;

impl Command for EncodeBase32 {
    fn name(&self) -> &str {
        "encode base32"
    }

    fn signature(&self) -> Signature {
        Signature::build("encode base32")
            .input_output_types(vec![
                (Type::String, Type::String),
                (Type::Binary, Type::String),
            ])
            .switch("nopad", "Don't accept padding.", None)
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Encode a string or binary value using Base32."
    }

    fn extra_description(&self) -> &str {
        EXTRA_USAGE
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Encode a binary value",
                example: r#"0x[01 02 10] | encode base32"#,
                result: Some(Value::test_string("AEBBA===")),
            },
            Example {
                description: "Encode a string",
                example: r#""hello there" | encode base32"#,
                result: Some(Value::test_string("NBSWY3DPEB2GQZLSMU======")),
            },
            Example {
                description: "Don't apply padding to the output",
                example: r#""hi" | encode base32 --nopad"#,
                result: Some(Value::test_string("NBUQ")),
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
        let encoding = if call.has_flag(engine_state, stack, "nopad")? {
            data_encoding::BASE32_NOPAD
        } else {
            data_encoding::BASE32
        };
        super::encode(encoding, call.span(), input)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let encoding = if call.has_flag_const(working_set, "nopad")? {
            data_encoding::BASE32_NOPAD
        } else {
            data_encoding::BASE32
        };
        super::encode(encoding, call.span(), input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples_decode() {
        crate::test_examples(DecodeBase32)
    }

    #[test]
    fn test_examples_encode() {
        crate::test_examples(EncodeBase32)
    }
}
