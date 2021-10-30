use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{PipelineData, ShellError, Signature};

#[derive(Clone)]
pub struct RunPlugin;

impl Command for RunPlugin {
    fn name(&self) -> &str {
        "run_plugin"
    }

    fn usage(&self) -> &str {
        "test for plugin encoding"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("run_plugin")
    }

    fn run(
        &self,
        _context: &EngineState,
        _stack: &mut Stack,
        _call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        Err(ShellError::InternalError("plugin".into()))
    }
}
