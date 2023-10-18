use std::time::{Duration, UNIX_EPOCH};

use chrono::{DateTime, Utc};
use nu_protocol::engine::Command;
use nu_protocol::{PipelineData, ShellError, Signature, Type};
use reedline::{
    FileBackedHistory, History as ReedlineHistory, HistoryItem, SearchDirection, SearchQuery,
    SqliteBackedHistory,
};

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
            let ctrlc = engine_state.ctrlc.clone();
            let mut plaintext_history_path = config_path.clone();
            let mut sqlite_history_path = config_path;
            plaintext_history_path.push("nushell");
            plaintext_history_path.push("history.txt");
            sqlite_history_path.push("nushell/history.sqlite3");

            let plaintext_history_reader = FileBackedHistory::with_file(
                engine_state.config.max_history_size as usize,
                plaintext_history_path.clone(),
            )
            .map(|inner| {
                let boxed: Box<dyn ReedlineHistory> = Box::new(inner);
                boxed
            })
            .ok();
            let mut sqlite_history_reader =
                SqliteBackedHistory::with_file(sqlite_history_path.clone(), None, None)
                    .map(|inner| {
                        let boxed: Box<dyn ReedlineHistory> = Box::new(inner);
                        boxed
                    })
                    .ok()
                    .expect("SQLite history not found");

            plaintext_history_reader
                .and_then(|h| {
                    h.search(SearchQuery::everything(SearchDirection::Forward, None))
                        .ok()
                })
                .unwrap()
                .into_iter()
                .for_each(|entry| {
                    let mut history_item = HistoryItem::from_command_line(entry.command_line);
                    history_item.start_timestamp = Some(DateTime::<Utc>::from(UNIX_EPOCH));
                    let _history_item = sqlite_history_reader.save(history_item);
                });

            Ok(PipelineData::empty())
        } else {
            Err(ShellError::FileNotFound(head))
        }
    }
}
