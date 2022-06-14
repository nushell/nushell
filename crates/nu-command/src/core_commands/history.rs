use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, HistoryFileFormat, IntoInterruptiblePipelineData, PipelineData, ShellError,
    Signature, Value,
};
use reedline::{
    FileBackedHistory, History as ReedlineHistory, SearchDirection, SearchQuery,
    SqliteBackedHistory,
};

#[derive(Clone)]
pub struct History;

impl Command for History {
    fn name(&self) -> &str {
        "history"
    }

    fn usage(&self) -> &str {
        "Get the command history"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("history")
            .switch("clear", "Clears out the history entries", Some('c'))
            .category(Category::Core)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let head = call.head;
        // todo for sqlite history this command should be an alias to `open ~/.config/nushell/history.sqlite3 | get history`
        if let Some(config_path) = nu_path::config_dir() {
            let clear = call.has_flag("clear");
            let ctrlc = engine_state.ctrlc.clone();

            let mut history_path = config_path;
            history_path.push("nushell");
            match engine_state.config.history_file_format {
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
                Ok(PipelineData::new(head))
            } else {
                let history_reader: Option<Box<dyn ReedlineHistory>> =
                    match engine_state.config.history_file_format {
                        HistoryFileFormat::Sqlite => SqliteBackedHistory::with_file(history_path)
                            .map(|inner| {
                                let boxed: Box<dyn ReedlineHistory> = Box::new(inner);
                                boxed
                            })
                            .ok(),

                        HistoryFileFormat::PlainText => FileBackedHistory::with_file(
                            engine_state.config.max_history_size as usize,
                            history_path,
                        )
                        .map(|inner| {
                            let boxed: Box<dyn ReedlineHistory> = Box::new(inner);
                            boxed
                        })
                        .ok(),
                    };

                let data = history_reader
                    .and_then(|h| {
                        h.search(SearchQuery::everything(SearchDirection::Forward))
                            .ok()
                    })
                    .map(move |entries| {
                        entries
                            .into_iter()
                            .enumerate()
                            .map(move |(idx, entry)| Value::Record {
                                cols: vec!["command".to_string(), "index".to_string()],
                                vals: vec![
                                    Value::String {
                                        val: entry.command_line,
                                        span: head,
                                    },
                                    Value::Int {
                                        val: idx as i64,
                                        span: head,
                                    },
                                ],
                                span: head,
                            })
                    })
                    .ok_or(ShellError::FileNotFound(head))?
                    .into_pipeline_data(ctrlc);
                Ok(data)
            }
        } else {
            Err(ShellError::FileNotFound(head))
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
                example: "history | wrap cmd | where cmd =~ cargo",
                description: "Search all the commands from history that contains 'cargo'",
                result: None,
            },
        ]
    }
}
