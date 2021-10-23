use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{ShellError, Signature, Value};

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
        _context: &EvaluationContext,
        _call: &Call,
        _input: Value,
    ) -> Result<nu_protocol::Value, ShellError> {
        Err(ShellError::InternalError("plugin".into()))
    }
}
