use nu_engine::command_prelude::*;

const EXTRA_USAGE: &str = r"This command uses an alternative Base32 alphabet, defined in RFC 4648, section 7.

Note this command will collect stream input.";

#[derive(Clone)]
pub struct DecodeBase32Hex;

impl Command for DecodeBase32Hex {
    fn name(&self) -> &str {
        "decode base32hex"
    }

    fn signature(&self) -> Signature {
        Signature::build("decode base32hex")
            .input_output_types(vec![(Type::String, Type::Binary)])
            .allow_variants_without_examples(true)
            .switch("nopad", "Reject input with padding.", None)
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Encode a base32hex value."
    }

    fn extra_description(&self) -> &str {
        EXTRA_USAGE
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Decode arbitrary binary data",
                example: r#""ATNAQ===" | decode base32hex"#,
                result: Some(Value::test_binary(vec![0x57, 0x6E, 0xAD])),
            },
            Example {
                description: "Decode an encoded string",
                example: r#""D1KG====" | decode base32hex | decode"#,
                result: None,
            },
            Example {
                description: "Parse a string without padding",
                example: r#""ATNAQ" | decode base32hex --nopad"#,
                result: Some(Value::test_binary(vec![0x57, 0x6E, 0xAD])),
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
            data_encoding::BASE32HEX_NOPAD
        } else {
            data_encoding::BASE32HEX
        };

        super::decode(encoding, call.head, input)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let encoding = if call.has_flag_const(working_set, "nopad")? {
            data_encoding::BASE32HEX_NOPAD
        } else {
            data_encoding::BASE32HEX
        };

        super::decode(encoding, call.head, input)
    }
}

#[derive(Clone)]
pub struct EncodeBase32Hex;

impl Command for EncodeBase32Hex {
    fn name(&self) -> &str {
        "encode base32hex"
    }

    fn signature(&self) -> Signature {
        Signature::build("encode base32hex")
            .input_output_types(vec![
                (Type::String, Type::String),
                (Type::Binary, Type::String),
            ])
            .switch("nopad", "Don't pad the output.", None)
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Encode a binary value or a string using base32hex."
    }

    fn extra_description(&self) -> &str {
        EXTRA_USAGE
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Encode a binary value",
                example: r#"0x[57 6E AD] | encode base32hex"#,
                result: Some(Value::test_string("ATNAQ===")),
            },
            Example {
                description: "Encode a string",
                example: r#""hello there" | encode base32hex"#,
                result: Some(Value::test_string("D1IMOR3F41Q6GPBICK======")),
            },
            Example {
                description: "Don't apply padding to the output",
                example: r#""hello there" | encode base32hex --nopad"#,
                result: Some(Value::test_string("D1IMOR3F41Q6GPBICK")),
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
            data_encoding::BASE32HEX_NOPAD
        } else {
            data_encoding::BASE32HEX
        };

        super::encode(encoding, call.head, input)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let encoding = if call.has_flag_const(working_set, "nopad")? {
            data_encoding::BASE32HEX_NOPAD
        } else {
            data_encoding::BASE32HEX
        };

        super::encode(encoding, call.head, input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples_decode() {
        crate::test_examples(DecodeBase32Hex)
    }
    #[test]
    fn test_examples_encode() {
        crate::test_examples(EncodeBase32Hex)
    }
}
