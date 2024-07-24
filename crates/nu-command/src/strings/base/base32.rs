use data_encoding::Encoding;

use nu_engine::command_prelude::*;

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

    fn usage(&self) -> &str {
        "Decode a value."
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
            .allow_variants_without_examples(true)
            .switch("nopad", "Don't accept padding.", None)
            .switch("dnscurve", "Parse as the DNSCURVE Base32 variant.", None)
            .switch("dnssec", "Parse as the DNSSEC Base32 variant.", None)
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
