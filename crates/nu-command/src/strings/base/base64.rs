use nu_engine::command_prelude::*;

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
            .required("encoding", SyntaxShape::String, "encoding to use")
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
            .allow_variants_without_examples(true)
            .required("encoding", SyntaxShape::String, "encoding to use")
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
    fn test_examples_decode() {
        crate::test_examples(DecodeBase64)
    }

    #[test]
    fn test_examples_encode() {
        crate::test_examples(EncodeBase64)
    }
}
