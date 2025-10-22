use super::{config_update_string_enum, prelude::*};
use crate::{self as nu_protocol, ConfigWarning};

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
            #[cfg(feature = "sqlite")]
            _ => Err("'sqlite' or 'plaintext'"),
            #[cfg(not(feature = "sqlite"))]
            _ => Err("'plaintext'"),
        }
    }
}

impl UpdateFromValue for HistoryFileFormat {
    fn update(&mut self, value: &Value, path: &mut ConfigPath, errors: &mut ConfigErrors) {
        config_update_string_enum(self, value, path, errors);

        #[cfg(not(feature = "sqlite"))]
        if *self == HistoryFileFormat::Sqlite {
            *self = HistoryFileFormat::Plaintext;
            errors.warn(ConfigWarning::IncompatibleOptions {
                label: "SQLite-based history file only supported with the `sqlite` feature, falling back to plain text history", 
                span: value.span(),
                help: "Compile Nushell with `sqlite` feature enabled",
            });
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

        // might not be correct if file format was changed away from sqlite rather than isolation,
        // but this is an edge case and the span of the relevant value here should be close enough
        let mut isolation_span = value.span();

        for (col, val) in record.iter() {
            let path = &mut path.push(col);
            match col.as_str() {
                "isolation" => {
                    isolation_span = val.span();
                    self.isolation.update(val, path, errors)
                }
                "sync_on_enter" => self.sync_on_enter.update(val, path, errors),
                "max_size" => self.max_size.update(val, path, errors),
                "file_format" => self.file_format.update(val, path, errors),
                _ => errors.unknown_option(path, val),
            }
        }

        // Listing all formats separately in case additional ones are added
        match (self.isolation, self.file_format) {
            (true, HistoryFileFormat::Plaintext) => {
                errors.warn(ConfigWarning::IncompatibleOptions {
                    label: "history isolation only compatible with SQLite format",
                    span: isolation_span,
                    help: r#"disable history isolation, or set $env.config.history.file_format = "sqlite""#,
                });
            }
            (true, HistoryFileFormat::Sqlite) => (),
            (false, _) => (),
        }
    }
}
