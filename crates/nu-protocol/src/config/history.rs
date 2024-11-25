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

#[derive(Clone, Debug, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryConfig {
    pub max_size: i64,
    pub sync_on_enter: bool,
    pub file_format: HistoryFileFormat,
    pub isolation: bool,
    pub path: Option<String>,
}

impl HistoryConfig {
    pub fn file_path(&self, call_head: Option<Span>) -> Result<std::path::PathBuf, ShellError> {
        match self.path.clone() {
            None => self.system_defined_file_path(call_head),
            Some(path) => self.user_defined_file_path(path, call_head),
        }
    }

    fn system_defined_file_path(
        &self,
        call_head: Option<Span>,
    ) -> Result<std::path::PathBuf, ShellError> {
        let system_path = nu_path::nu_config_dir().map(|mut history_path| {
            history_path.push(self.file_format.default_file_name());
            history_path.into()
        });
        match system_path {
            Some(path) => Ok(path),
            None => Err(ShellError::ConfigDirNotFound { span: call_head }),
        }
    }

    fn user_defined_file_path(
        &self,
        path_from_config: String,
        call_head: Option<Span>,
    ) -> Result<std::path::PathBuf, ShellError> {
        let error = Err(ShellError::HistoryDirNotFound { span: call_head });

        let user_path = std::path::Path::new(&path_from_config);
        let dir_path = match user_path.parent() {
            Some(path) => path,
            None => return error,
        };

        match dir_path.exists() {
            true => Ok(user_path.into()),
            false => error,
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
            path: None,
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
                "path" => match val {
                    Value::Nothing { .. } => self.path = None,
                    Value::String { val, .. } => self.path = Some(val.clone()),
                    _ => errors.type_mismatch(path, Type::custom("path or nothing"), val),
                },
                _ => errors.unknown_option(path, val),
            }
        }
    }
}
