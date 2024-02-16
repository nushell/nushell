use nu_protocol::HistoryFileFormat;
use reedline::{FileBackedHistory, History as ReedlineHistory, SqliteBackedHistory};

pub(super) fn reedline_history(
    history: nu_protocol::HistoryConfig,
    history_path: std::path::PathBuf,
) -> Option<Box<dyn ReedlineHistory>> {
    match history.file_format {
        HistoryFileFormat::Sqlite => SqliteBackedHistory::with_file(history_path, None, None)
            .map(|inner| {
                let boxed: Box<dyn ReedlineHistory> = Box::new(inner);
                boxed
            })
            .ok(),

        HistoryFileFormat::PlainText => {
            FileBackedHistory::with_file(history.max_size as usize, history_path)
                .map(|inner| {
                    let boxed: Box<dyn ReedlineHistory> = Box::new(inner);
                    boxed
                })
                .ok()
        }
    }
}

pub(super) fn history_path(
    config_path: std::path::PathBuf,
    history: nu_protocol::HistoryConfig,
) -> std::path::PathBuf {
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
    history_path
}
