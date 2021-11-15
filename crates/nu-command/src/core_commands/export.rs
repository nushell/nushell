use nu_engine::get_full_help;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    IntoPipelineData, PipelineData, Signature, Value,
};

#[derive(Clone)]
pub struct ExportCommand;

impl Command for ExportCommand {
    fn name(&self) -> &str {
        "export"
    }

    fn signature(&self) -> Signature {
        Signature::build("export")
    }

    fn usage(&self) -> &str {
        "Export custom commands or environment variables from a module."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        Ok(Value::String {
            val: get_full_help(
                &ExportCommand.signature(),
                &ExportCommand.examples(),
                engine_state,
            ),
            span: call.head,
        }
        .into_pipeline_data())
    }
}
