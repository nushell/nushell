use nu_engine::{command_prelude::*, env_to_strings};

#[derive(Clone)]
pub struct DebugEnv;

impl Command for DebugEnv {
    fn name(&self) -> &str {
        "debug env"
    }

    fn signature(&self) -> Signature {
        Signature::new(self.name())
            .input_output_type(Type::Nothing, Type::record())
            .category(Category::Debug)
    }

    fn description(&self) -> &str {
        "Show environment variables as external commands would get it."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(PipelineData::Value(
            env_to_strings(engine_state, stack)?.into_value(call.head),
            None,
        ))
    }
}
