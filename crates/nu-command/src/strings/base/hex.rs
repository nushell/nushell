use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct DecodeHex;

impl Command for DecodeHex {
    fn name(&self) -> &str {
        "decode hex"
    }

    fn signature(&self) -> Signature {
        Signature::build("decode hex")
            .input_output_types(vec![(Type::String, Type::Binary)])
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Hex decode a value."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Decode arbitrary binary data",
                example: r#""09FD" | decode hex"#,
                result: Some(Value::test_binary(vec![0x09, 0xFD])),
            },
            Example {
                description: "Lowercase Hex is also accepted",
                example: r#""09fd" | decode hex"#,
                result: Some(Value::test_binary(vec![0x09, 0xFD])),
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
        super::decode(data_encoding::HEXLOWER_PERMISSIVE, call.head, input)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        super::decode(data_encoding::HEXLOWER_PERMISSIVE, call.span(), input)
    }
}

#[derive(Clone)]
pub struct EncodeHex;

impl Command for EncodeHex {
    fn name(&self) -> &str {
        "encode hex"
    }

    fn signature(&self) -> Signature {
        Signature::build("encode hex")
            .input_output_types(vec![
                (Type::String, Type::String),
                (Type::Binary, Type::String),
            ])
            .switch("lower", "Encode to lowercase hex.", None)
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Hex encode a binary value or a string."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Encode a binary value",
                example: r#"0x[C3 06] | encode hex"#,
                result: Some(Value::test_string("C306")),
            },
            Example {
                description: "Encode a string",
                example: r#""hello" | encode hex"#,
                result: Some(Value::test_string("68656C6C6F")),
            },
            Example {
                description: "Output a Lowercase version of the encoding",
                example: r#"0x[AD EF] | encode hex --lower"#,
                result: Some(Value::test_string("adef")),
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
        let encoding = if call.has_flag(engine_state, stack, "lower")? {
            data_encoding::HEXLOWER
        } else {
            data_encoding::HEXUPPER
        };

        super::encode(encoding, call.head, input)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let encoding = if call.has_flag_const(working_set, "lower")? {
            data_encoding::HEXLOWER
        } else {
            data_encoding::HEXUPPER
        };

        super::encode(encoding, call.head, input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples_decode() {
        crate::test_examples(DecodeHex)
    }

    #[test]
    fn test_examples_encode() {
        crate::test_examples(EncodeHex)
    }
}
