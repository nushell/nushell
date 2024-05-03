use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct SysTemp;

impl Command for SysTemp {
    fn name(&self) -> &str {
        "sys temp"
    }

    fn signature(&self) -> Signature {
        Signature::build("sys temp")
            .filter()
            .category(Category::System)
            .input_output_types(vec![(Type::Nothing, Type::table())])
    }

    fn usage(&self) -> &str {
        "View the temperatures of system components."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(super::temp(call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Show the system temperatures",
            example: "sys temp",
            result: None,
        }]
    }
}
