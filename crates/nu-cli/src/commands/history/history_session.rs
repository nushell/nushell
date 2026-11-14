use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct HistorySession;

impl Command for HistorySession {
    fn name(&self) -> &str {
        "history session"
    }

    fn description(&self) -> &str {
        "Get the current history session id (0 when history isolation is off)."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("history session")
            .category(Category::History)
            .input_output_types(vec![(Type::Nothing, Type::Int)])
    }

    fn examples(&self) -> Vec<Example<'_>> {
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
    ) -> Result<PipelineData, ShellError> {
        // The id is only recorded on history rows when isolation is on; report
        // 0 otherwise so callers can't mistake the startup id for a real session.
        let session_id = match engine_state.history_config() {
            Some(history) if history.isolation => engine_state.history_session_id,
            _ => 0,
        };
        Ok(Value::int(session_id, call.head).into_pipeline_data())
    }
}
