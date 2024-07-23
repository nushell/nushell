use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct EncodeBase;

impl Command for EncodeBase {
    fn name(&self) -> &str {
        "encode base"
    }

    fn signature(&self) -> Signature {
        Signature::build("encode base")
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
        let name: String = call.req(engine_state, stack, 0)?;

        encode(&name, call.span(), input)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let name: String = call.req_const(working_set, 0)?;

        encode(&name, call.span(), input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        crate::test_examples(EncodeBase)
    }
}
