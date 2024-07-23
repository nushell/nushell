use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct DecodeBase32Hex;

impl Command for DecodeBase32Hex {
    fn name(&self) -> &str {
        "decode base32hex"
    }

    fn signature(&self) -> Signature {
        Signature::build("decode base32hex")
            .input_output_types(vec![
                (Type::String, Type::String),
                (Type::Binary, Type::String),
            ])
            .allow_variants_without_examples(true)
            .switch("nopad", "Reject input with padding.", None)
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "Encode a value."
    }

    fn extra_usage(&self) -> &str {
        "TODO"
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
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
            .allow_variants_without_examples(true)
            .switch("nopad", "Don't pad the output.", None)
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "Encode a value."
    }

    fn extra_usage(&self) -> &str {
        "TODO"
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
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
