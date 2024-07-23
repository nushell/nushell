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
            .switch("lower", "Use a lowercase version of Base32.", None)
            .switch("nopad", "Do not pad the output.", None)
            .switch("dnscurve", "Use DNSCURVE Base32 variant.", None)
            .switch("dnssec", "Use DNSSEC Base32 variant.", None)
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
        todo!()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        crate::test_examples(DecodeBase32)
    }
}
