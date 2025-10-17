use nu_engine::command_prelude::*;
use nu_protocol::{
    HistoryFileFormat,
    shell_error::{self, io::IoError},
};
use reedline::{FileBackedHistory, History as ReedlineHistory, SearchDirection, SearchQuery};
#[cfg(feature = "sqlite")]
use reedline::{HistoryItem, SqliteBackedHistory};

use super::fields;

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
            .switch("clear", "Clears out the history entries", Some('c'))
            .switch(
                "long",
                "Show long listing of entries for sqlite history",
                Some('l'),
            )
            .category(Category::History)
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
            HistoryFileFormat::Sqlite => Ok(history_reader
                .and_then(|h| {
                    h.search(SearchQuery::everything(SearchDirection::Forward, None))
                        .ok()
                })
                .map(move |entries| {
                    entries.into_iter().enumerate().map(move |(idx, entry)| {
                        create_sqlite_history_record(idx, entry, long, head)
                    })
                })
                .ok_or(IoError::new(
                    shell_error::io::ErrorKind::FileNotFound,
                    head,
                    history_path,
                ))?
                .into_pipeline_data(head, signals)),
        }
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
                example: "history | where command =~ cargo | get command",
                description: "Search all the commands from history that contains 'cargo'",
                result: None,
            },
        ]
    }
}

#[cfg(feature = "sqlite")]
fn create_sqlite_history_record(idx: usize, entry: HistoryItem, long: bool, head: Span) -> Value {
    //1. Format all the values
    //2. Create a record of either short or long columns and values

    let item_id_value = Value::int(
        entry
            .id
            .and_then(|id| id.to_string().parse::<i64>().ok())
            .unwrap_or_default(),
        head,
    );
    let start_timestamp_value = Value::date(
        entry.start_timestamp.unwrap_or_default().fixed_offset(),
        head,
    );
    let command_value = Value::string(entry.command_line, head);
    let session_id_value = Value::int(
        entry
            .session_id
            .and_then(|id| id.to_string().parse::<i64>().ok())
            .unwrap_or_default(),
        head,
    );
    let hostname_value = Value::string(entry.hostname.unwrap_or_default(), head);
    let cwd_value = Value::string(entry.cwd.unwrap_or_default(), head);
    let duration_value = Value::duration(
        entry
            .duration
            .and_then(|d| d.as_nanos().try_into().ok())
            .unwrap_or(0),
        head,
    );
    let exit_status_value = Value::int(entry.exit_status.unwrap_or(0), head);
    let index_value = Value::int(idx as i64, head);
    if long {
        Value::record(
            record! {
                "item_id" => item_id_value,
                fields::START_TIMESTAMP => start_timestamp_value,
                fields::COMMAND_LINE => command_value,
                fields::SESSION_ID => session_id_value,
                fields::HOSTNAME => hostname_value,
                fields::CWD => cwd_value,
                fields::DURATION => duration_value,
                fields::EXIT_STATUS => exit_status_value,
                "idx" => index_value,
            },
            head,
        )
    } else {
        Value::record(
            record! {
                fields::START_TIMESTAMP => start_timestamp_value,
                fields::COMMAND_LINE => command_value,
                fields::CWD => cwd_value,
                fields::DURATION => duration_value,
                fields::EXIT_STATUS => exit_status_value,
            },
            head,
        )
    }
}
