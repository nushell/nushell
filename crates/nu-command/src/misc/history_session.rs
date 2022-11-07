use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, IntoPipelineData, PipelineData, Signature, Value};

#[derive(Clone)]
pub struct HistorySession;

impl Command for HistorySession {
    fn name(&self) -> &str {
        "history session"
    }

    fn usage(&self) -> &str {
        "Get the command history session"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("history session").category(Category::Misc)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            example: "history session",
            description: "Get current history session",
            result: None,
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        Ok(Value::int(engine_state.history_session_id, call.head).into_pipeline_data())
    }
}
