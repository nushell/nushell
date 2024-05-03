use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct SysMem;

impl Command for SysMem {
    fn name(&self) -> &str {
        "sys mem"
    }

    fn signature(&self) -> Signature {
        Signature::build("sys mem")
            .filter()
            .category(Category::System)
            .input_output_types(vec![(Type::Nothing, Type::record())])
    }

    fn usage(&self) -> &str {
        "View information about the system memory."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(super::mem(call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        todo!()
    }
}
