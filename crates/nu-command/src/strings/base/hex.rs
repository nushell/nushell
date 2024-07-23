use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct DecodeHex;

impl Command for DecodeHex {
    fn name(&self) -> &str {
        "decode base"
    }

    fn signature(&self) -> Signature {
        Signature::build("decode base")
            .input_output_types(vec![(Type::String, Type::Binary)])
            .allow_variants_without_examples(true)
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "TODO"
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
        super::decode(data_encoding::HEXLOWER_PERMISSIVE, call.head, input)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        todo!()
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
            .allow_variants_without_examples(true)
            .switch("lower", "Encode to lowercase hex.", None)
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "TODO"
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
