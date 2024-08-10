use super::prelude::*;
use crate as nu_protocol;

#[derive(Clone, Copy, Debug, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub enum HistoryFileFormat {
    /// Store history as an SQLite database with additional context
    Sqlite,
    /// store history as a plain text file where every line is one command (without any context such as timestamps)
    Plaintext,
}

impl HistoryFileFormat {
    pub fn default_file_name(self) -> std::path::PathBuf {
        match self {
            HistoryFileFormat::Plaintext => "history.txt",
            HistoryFileFormat::Sqlite => "history.sqlite3",
        }
        .into()
    }
}

impl FromStr for HistoryFileFormat {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "sqlite" => Ok(Self::Sqlite),
            "plaintext" => Ok(Self::Plaintext),
            _ => Err("expected either 'sqlite' or 'plaintext'"),
        }
    }
}

#[derive(Clone, Copy, Debug, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryConfig {
    pub max_size: i64,
    pub sync_on_enter: bool,
    pub file_format: HistoryFileFormat,
    pub isolation: bool,
}

impl HistoryConfig {
    pub fn file_path(&self) -> Option<std::path::PathBuf> {
        nu_path::nu_config_dir().map(|mut history_path| {
            history_path.push(self.file_format.default_file_name());
            history_path.into()
        })
    }
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            max_size: 100_000,
            sync_on_enter: true,
            file_format: HistoryFileFormat::Plaintext,
            isolation: false,
        }
    }
}
