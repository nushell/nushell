use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct SysCpu;

impl Command for SysCpu {
    fn name(&self) -> &str {
        "sys cpu"
    }

    fn signature(&self) -> Signature {
        Signature::build("sys cpu")
            .filter()
            .category(Category::System)
            .input_output_types(vec![(Type::Nothing, Type::table())])
    }

    fn usage(&self) -> &str {
        "View information about the system CPUs."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(super::cpu(call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Show info about the system CPUs",
            example: "sys cpu",
            result: None,
        }]
    }
}
