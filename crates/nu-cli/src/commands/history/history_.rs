use nu_engine::command_prelude::*;
use nu_protocol::HistoryFileFormat;
use reedline::{
    FileBackedHistory, History as ReedlineHistory, HistoryItem, SearchDirection, SearchQuery,
    SqliteBackedHistory,
};

#[derive(Clone)]
pub struct History;

impl Command for History {
    fn name(&self) -> &str {
        "history"
    }

    fn usage(&self) -> &str {
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
        if let Some(config_path) = nu_path::config_dir() {
            let clear = call.has_flag(engine_state, stack, "clear")?;
            let long = call.has_flag(engine_state, stack, "long")?;
            let ctrlc = engine_state.ctrlc.clone();

            let mut history_path = config_path;
            history_path.push("nushell");
            match history.file_format {
                HistoryFileFormat::Sqlite => {
                    history_path.push("history.sqlite3");
                }
                HistoryFileFormat::PlainText => {
                    history_path.push("history.txt");
                }
            }

            if clear {
                let _ = std::fs::remove_file(history_path);
                // TODO: FIXME also clear the auxiliary files when using sqlite
                Ok(PipelineData::empty())
            } else {
                let history_reader: Option<Box<dyn ReedlineHistory>> = match history.file_format {
                    HistoryFileFormat::Sqlite => {
                        SqliteBackedHistory::with_file(history_path.clone(), None, None)
                            .map(|inner| {
                                let boxed: Box<dyn ReedlineHistory> = Box::new(inner);
                                boxed
                            })
                            .ok()
                    }

                    HistoryFileFormat::PlainText => FileBackedHistory::with_file(
                        history.max_size as usize,
                        history_path.clone(),
                    )
                    .map(|inner| {
                        let boxed: Box<dyn ReedlineHistory> = Box::new(inner);
                        boxed
                    })
                    .ok(),
                };

                match history.file_format {
                    HistoryFileFormat::PlainText => Ok(history_reader
                        .and_then(|h| {
                            h.search(SearchQuery::everything(SearchDirection::Forward, None))
                                .ok()
                        })
                        .map(move |entries| {
                            entries.into_iter().enumerate().map(move |(idx, entry)| {
                                Value::record(
                                    record! {
                                        "command" => Value::string(entry.command_line, head),
                                        "index" => Value::int(idx as i64, head),
                                    },
                                    head,
                                )
                            })
                        })
                        .ok_or(ShellError::FileNotFound {
                            file: history_path.display().to_string(),
                            span: head,
                        })?
                        .into_pipeline_data(ctrlc)),
                    HistoryFileFormat::Sqlite => Ok(history_reader
                        .and_then(|h| {
                            h.search(SearchQuery::everything(SearchDirection::Forward, None))
                                .ok()
                        })
                        .map(move |entries| {
                            entries.into_iter().enumerate().map(move |(idx, entry)| {
                                create_history_record(idx, entry, long, head)
                            })
                        })
                        .ok_or(ShellError::FileNotFound {
                            file: history_path.display().to_string(),
                            span: head,
                        })?
                        .into_pipeline_data(ctrlc)),
                }
            }
        } else {
            Err(ShellError::ConfigDirNotFound { span: Some(head) })
        }
    }

    fn examples(&self) -> Vec<Example> {
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

fn create_history_record(idx: usize, entry: HistoryItem, long: bool, head: Span) -> Value {
    //1. Format all the values
    //2. Create a record of either short or long columns and values

    let item_id_value = Value::int(
        match entry.id {
            Some(id) => {
                let ids = id.to_string();
                match ids.parse::<i64>() {
                    Ok(i) => i,
                    _ => 0i64,
                }
            }
            None => 0i64,
        },
        head,
    );
    let start_timestamp_value = Value::string(
        match entry.start_timestamp {
            Some(time) => time.to_string(),
            None => "".into(),
        },
        head,
    );
    let command_value = Value::string(entry.command_line, head);
    let session_id_value = Value::int(
        match entry.session_id {
            Some(sid) => {
                let sids = sid.to_string();
                match sids.parse::<i64>() {
                    Ok(i) => i,
                    _ => 0i64,
                }
            }
            None => 0i64,
        },
        head,
    );
    let hostname_value = Value::string(
        match entry.hostname {
            Some(host) => host,
            None => "".into(),
        },
        head,
    );
    let cwd_value = Value::string(
        match entry.cwd {
            Some(cwd) => cwd,
            None => "".into(),
        },
        head,
    );
    let duration_value = Value::duration(
        match entry.duration {
            Some(d) => d.as_nanos().try_into().unwrap_or(0),
            None => 0,
        },
        head,
    );
    let exit_status_value = Value::int(entry.exit_status.unwrap_or(0), head);
    let index_value = Value::int(idx as i64, head);
    if long {
        Value::record(
            record! {
                "item_id" => item_id_value,
                "start_timestamp" => start_timestamp_value,
                "command" => command_value,
                "session_id" => session_id_value,
                "hostname" => hostname_value,
                "cwd" => cwd_value,
                "duration" => duration_value,
                "exit_status" => exit_status_value,
                "idx" => index_value,
            },
            head,
        )
    } else {
        Value::record(
            record! {
                "start_timestamp" => start_timestamp_value,
                "command" => command_value,
                "cwd" => cwd_value,
                "duration" => duration_value,
                "exit_status" => exit_status_value,
            },
            head,
        )
    }
}
