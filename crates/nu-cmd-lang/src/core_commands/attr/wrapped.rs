use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct AttrWrapped;

impl Command for AttrWrapped {
    fn name(&self) -> &str {
        "attr wrapped"
    }

    fn signature(&self) -> Signature {
        Signature::build("attr wrapped")
            .input_output_type(Type::Nothing, Type::Nothing)
            .category(Category::Core)
    }

    fn description(&self) -> &str {
        "Attribute for making custom commands treat unknown flags and arguments as strings."
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
