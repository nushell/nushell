use super::{config_update_string_enum, prelude::*};
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
            _ => Err("'sqlite' or 'plaintext'"),
        }
    }
}

impl UpdateFromValue for HistoryFileFormat {
    fn update(&mut self, value: &Value, path: &mut ConfigPath, errors: &mut ConfigErrors) {
        config_update_string_enum(self, value, path, errors)
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
        let history_path: std::path::PathBuf = nu_path::nu_data_dir().map(|mut history_path| {
            history_path.push(self.file_format.default_file_name());
            history_path.into()
        })?;

        self.maybe_migrate_history_file_path(history_path.clone());

        Some(history_path)
    }

    fn maybe_migrate_history_file_path(&self, modern_history_path: std::path::PathBuf) {
        let maybe_pre_0_99_1_history_path: Option<std::path::PathBuf> = nu_path::nu_config_dir()
            .map(|mut path| {
                path.push(self.file_format.default_file_name());
                path.into()
            });

        let Some(pre_0_99_1_history_path) = maybe_pre_0_99_1_history_path else {
            return;
        };

        if modern_history_path == pre_0_99_1_history_path {
            return;
        }

        if !pre_0_99_1_history_path.exists() || modern_history_path.exists() {
            return;
        }

        // TODO: Create the base directory? `std::fs::create_dir(modern_history_path.parent())`
        log::info!("Moving {pre_0_99_1_history_path:?} to {modern_history_path:?}");
        let result = std::fs::rename(pre_0_99_1_history_path.clone(), modern_history_path.clone());

        if result.is_err() {
            // TODO: Report an error.
            //   It seems a shame to create a whole new error for something that isn't going to
            //   be relevant for the lifetime of Nushell. But panicking seems a bit overkill for an
            //   innocent migration.
            log::warn!("Couldn't migrate {pre_0_99_1_history_path:?} to {modern_history_path:?}. Error: {result:?}");
        }
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

impl UpdateFromValue for HistoryConfig {
    fn update<'a>(
        &mut self,
        value: &'a Value,
        path: &mut ConfigPath<'a>,
        errors: &mut ConfigErrors,
    ) {
        let Value::Record { val: record, .. } = value else {
            errors.type_mismatch(path, Type::record(), value);
            return;
        };

        for (col, val) in record.iter() {
            let path = &mut path.push(col);
            match col.as_str() {
                "isolation" => self.isolation.update(val, path, errors),
                "sync_on_enter" => self.sync_on_enter.update(val, path, errors),
                "max_size" => self.max_size.update(val, path, errors),
                "file_format" => self.file_format.update(val, path, errors),
                _ => errors.unknown_option(path, val),
            }
        }
    }
}
