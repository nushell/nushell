use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct SysNet;

impl Command for SysNet {
    fn name(&self) -> &str {
        "sys net"
    }

    fn signature(&self) -> Signature {
        Signature::build("sys net")
            .filter()
            .category(Category::System)
            .input_output_types(vec![(Type::Nothing, Type::table())])
    }

    fn usage(&self) -> &str {
        "View information about the system network interfaces."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(super::net(call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Show info about the system network",
            example: "sys net",
            result: None,
        }]
    }
}
