use super::fields;
use nu_engine::command_prelude::*;
use nu_protocol::{
    HistoryFileFormat,
    shell_error::{self, io::IoError},
};
#[cfg(feature = "sqlite")]
use reedline::SqliteBackedHistory;
use reedline::{FileBackedHistory, History as ReedlineHistory, SearchDirection, SearchQuery};

#[derive(Clone)]
pub struct History;

impl Command for History {
    fn name(&self) -> &str {
        "history"
    }

    fn description(&self) -> &str {
        "Get the command history."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("history")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .allow_variants_without_examples(true)
            .switch("clear", "Clears out the history entries.", Some('c'))
            .switch(
                "long",
                "Show long format with timestamps and additional details.",
                Some('l'),
            )
            .category(Category::History)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "history | length",
                description: "Get current history length",
                result: None,
            },
            Example {
                example: "history | last 5",
                description: "Show last 5 commands you have ran",
                result: None,
            },
            Example {
                example: "history --long | last 5",
                description: "Show last 5 commands with full details",
                result: None,
            },
            Example {
                example: "history | where command =~ cargo | get command",
                description: "Search all the commands from history that contains 'cargo'",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;

        let Some(history) = engine_state.history_config() else {
            return Ok(PipelineData::empty());
        };
        // todo for sqlite history this command should be an alias to `open ~/.config/nushell/history.sqlite3 | get history`
        let Some(history_path) = history.file_path() else {
            return Err(ShellError::ConfigDirNotFound { span: head });
        };

        if call.has_flag(engine_state, stack, "clear")? {
            let _ = std::fs::remove_file(history_path);
            // TODO: FIXME also clear the auxiliary files when using sqlite
            return Ok(PipelineData::empty());
        }

        #[cfg_attr(not(feature = "sqlite"), allow(unused_variables))]
        let long = call.has_flag(engine_state, stack, "long")?;

        let signals = engine_state.signals().clone();
        let history_reader: Option<Box<dyn ReedlineHistory>> = match history.file_format {
            #[cfg(feature = "sqlite")]
            HistoryFileFormat::Sqlite => {
                SqliteBackedHistory::with_file(history_path.clone(), None, None)
                    .map(|inner| {
                        let boxed: Box<dyn ReedlineHistory> = Box::new(inner);
                        boxed
                    })
                    .ok()
            }
            // this variant should never happen, the config value is handled in the `UpdateFromValue` impl
            #[cfg(not(feature = "sqlite"))]
            HistoryFileFormat::Sqlite => {
                return Err(ShellError::GenericError {
                    error: "Could not open history reader".into(),
                    msg: "SQLite is not supported".to_string(),
                    span: Some(call.head),
                    help: "Compile Nushell with `sqlite` feature".to_string().into(),
                    inner: vec![],
                });
            }
            HistoryFileFormat::Plaintext => {
                FileBackedHistory::with_file(history.max_size as usize, history_path.clone())
                    .map(|inner| {
                        let boxed: Box<dyn ReedlineHistory> = Box::new(inner);
                        boxed
                    })
                    .ok()
            }
        };
        match history.file_format {
            HistoryFileFormat::Plaintext => Ok(history_reader
                .and_then(|h| {
                    h.search(SearchQuery::everything(SearchDirection::Forward, None))
                        .ok()
                })
                .map(move |entries| {
                    entries.into_iter().enumerate().map(move |(idx, entry)| {
                        Value::record(
                            record! {
                                fields::COMMAND_LINE => Value::string(entry.command_line, head),
                                // TODO: This name is inconsistent with create_history_record.
                                "index" => Value::int(idx as i64, head),
                            },
                            head,
                        )
                    })
                })
                .ok_or(IoError::new(
                    shell_error::io::ErrorKind::FileNotFound,
                    head,
                    history_path,
                ))?
                .into_pipeline_data(head, signals)),
            // this variant should never happen, the config value is handled in the `UpdateFromValue` impl
            #[cfg(not(feature = "sqlite"))]
            HistoryFileFormat::Sqlite => {
                return Err(ShellError::GenericError {
                    error: "Could not open history reader".into(),
                    msg: "SQLite is not supported".to_string(),
                    span: Some(call.head),
                    help: "Compile Nushell with `sqlite` feature".to_string().into(),
                    inner: vec![],
                });
            }
            #[cfg(feature = "sqlite")]
            HistoryFileFormat::Sqlite => {
                // Return a lazy SQLiteQueryBuilder for the history table
                let mut table = nu_command::SQLiteQueryBuilder::new(
                    history_path,
                    "history".to_string(),
                    signals,
                );
                if long {
                    table = table.with_select("id as item_id, start_timestamp, command_line as command, session_id, hostname, cwd, duration_ms as duration, exit_status, rowid as idx".to_string());
                } else {
                    table = table.with_select(
                        "start_timestamp, command_line as command, cwd, duration_ms as duration, exit_status"
                            .to_string(),
                    );
                }
                Ok(PipelineData::Value(
                    Value::custom(Box::new(table), head),
                    None,
                ))
            }
        }
    }
}
