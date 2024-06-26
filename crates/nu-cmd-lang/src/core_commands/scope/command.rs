use nu_engine::{command_prelude::*, get_full_help};

#[derive(Clone)]
pub struct Scope;

impl Command for Scope {
    fn name(&self) -> &str {
        "scope"
    }

    fn signature(&self) -> Signature {
        Signature::build("scope")
            .category(Category::Core)
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .allow_variants_without_examples(true)
    }

    fn usage(&self) -> &str {
        "Commands for getting info about what is in scope."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(Value::string(get_full_help(self, engine_state, stack), call.head).into_pipeline_data())
    }
}
