use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct SysDisks;

impl Command for SysDisks {
    fn name(&self) -> &str {
        "sys disks"
    }

    fn signature(&self) -> Signature {
        Signature::build("sys disks")
            .filter()
            .category(Category::System)
            .input_output_types(vec![(Type::Nothing, Type::table())])
    }

    fn usage(&self) -> &str {
        "View information about the system disks."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(super::disks(call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Show info about the system disks",
            example: "sys disks",
            result: None,
        }]
    }
}
