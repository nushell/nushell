use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct AttrEnv;

impl Command for AttrEnv {
    fn name(&self) -> &str {
        "attr env"
    }

    fn signature(&self) -> Signature {
        Signature::build("attr env")
            .input_output_type(Type::Nothing, Type::Nothing)
            .category(Category::Core)
    }

    fn description(&self) -> &str {
        "Attribute for making custom commands keep environment defined inside the command."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(Value::nothing(call.head).into_pipeline_data())
    }

    fn run_const(
        &self,
        _working_set: &StateWorkingSet,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(Value::nothing(call.head).into_pipeline_data())
    }

    fn is_const(&self) -> bool {
        true
    }
}
