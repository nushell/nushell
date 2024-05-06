use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct SysHost;

impl Command for SysHost {
    fn name(&self) -> &str {
        "sys host"
    }

    fn signature(&self) -> Signature {
        Signature::build("sys host")
            .filter()
            .category(Category::System)
            .input_output_types(vec![(Type::Nothing, Type::record())])
    }

    fn usage(&self) -> &str {
        "View information about the system host."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(super::host(call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Show info about the system host",
            example: "sys host",
            result: None,
        }]
    }
}
