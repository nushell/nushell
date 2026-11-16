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
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // The id is only recorded on history rows when isolation is on; report
        // 0 otherwise so callers can't mistake the startup id for a real session.
        // Read the stack's config: `$env.config` changes made earlier in the same
        // config file aren't merged into engine_state until the file finishes.
        let history = &stack.get_config(engine_state).history;
        let session_id = if engine_state.history_enabled
            && history.isolation
            && history.path != nu_protocol::HistoryPath::Disabled
        {
            engine_state.history_session_id
        } else {
            0
        };
        Ok(Value::int(session_id, call.head).into_pipeline_data())
    }
}
