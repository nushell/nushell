use nu_protocol::engine::Command;
use nu_protocol::{PipelineData, ShellError, Signature, Type};

#[derive(Clone)]
pub struct HistoryMigrate;

impl Command for HistoryMigrate {
    fn name(&self) -> &str {
        "history migrate"
    }

    fn usage(&self) -> &str {
        "Migrate history from plain text to SQLite backend."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("history migrate").input_output_type(Type::Nothing, Type::Nothing)
    }

    fn run(
        &self,
        engine_state: &nu_protocol::engine::EngineState,
        stack: &mut nu_protocol::engine::Stack,
        call: &nu_protocol::ast::Call,
        input: nu_protocol::PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let head = call.head;

        if let Some(config_path) = nu_path::config_dir() {
            let mut plaintext_history_path = config_path.clone();
            let mut sqlite_history_path = config_path;
            plaintext_history_path.push("nushell/history.txt");
            sqlite_history_path.push("nushell/history.sqlite3");

            Ok(PipelineData::Empty)
        } else {
            Err(ShellError::FileNotFound(head))
        }
    }
}
