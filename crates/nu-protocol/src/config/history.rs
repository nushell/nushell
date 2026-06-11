use super::{config_update_string_enum, prelude::*};
use crate::{self as nu_protocol, ConfigWarning};
use std::path::PathBuf;

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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HistoryPath {
    Default,
    Custom(PathBuf),
    Disabled,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryConfig {
    pub max_size: i64,
    pub sync_on_enter: bool,
    pub file_format: HistoryFileFormat,
    pub isolation: bool,
    pub path: HistoryPath,
    pub ignore_space_prefixed: bool,
}

impl IntoValue for HistoryPath {
    fn into_value(self, span: Span) -> Value {
        match self {
            HistoryPath::Default => Value::string("", span),
            HistoryPath::Disabled => Value::nothing(span),
            HistoryPath::Custom(path) => Value::string(path.display().to_string(), span),
        }
    }
}

impl IntoValue for HistoryConfig {
    fn into_value(self, span: Span) -> Value {
        Value::record(
            record! {
                "max_size" => self.max_size.into_value(span),
                "sync_on_enter" => self.sync_on_enter.into_value(span),
                "file_format" => self.file_format.into_value(span),
                "isolation" => self.isolation.into_value(span),
                "path" => self.path.into_value(span),
                "ignore_space_prefixed" => self.ignore_space_prefixed.into_value(span),
            },
            span,
        )
    }
}

impl HistoryConfig {
    pub fn file_path(&self) -> Option<PathBuf> {
        let path = match &self.path {
            HistoryPath::Custom(path) => Some(path.clone()),
            HistoryPath::Disabled => None,
            HistoryPath::Default => nu_path::nu_config_dir().map(|mut history_path| {
                history_path.push(self.file_format.default_file_name());
                history_path.into()
            }),
        }?;

        if path.is_dir() {
            return Some(path.join(self.file_format.default_file_name()));
        }

        Some(path)
    }
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            max_size: 100_000,
            sync_on_enter: true,
            file_format: HistoryFileFormat::Plaintext,
            isolation: false,
            path: HistoryPath::Default,
            ignore_space_prefixed: true,
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
                    let prev = self.isolation;
                    self.isolation.update(val, path, errors);
                    if errors.history_locked_after_startup()
                        && self.isolation != errors.config().history.isolation
                    {
                        self.isolation = prev;
                        errors.locked_after_startup(path, val.span());
                    }
                }
                "sync_on_enter" => self.sync_on_enter.update(val, path, errors),
                "max_size" => {
                    let prev = self.max_size;
                    self.max_size.update(val, path, errors);
                    if errors.history_locked_after_startup()
                        && self.max_size != errors.config().history.max_size
                    {
                        self.max_size = prev;
                        errors.locked_after_startup(path, val.span());
                    }
                }
                "file_format" => {
                    let prev = self.file_format;
                    self.file_format.update(val, path, errors);
                    if errors.history_locked_after_startup()
                        && self.file_format != errors.config().history.file_format
                    {
                        self.file_format = prev;
                        errors.locked_after_startup(path, val.span());
                    }
                }
                "path" => match val {
                    Value::String { val: s, .. } => {
                        let new_path = if s.is_empty() {
                            HistoryPath::Default
                        } else {
                            HistoryPath::Custom(PathBuf::from(s))
                        };

                        if errors.history_locked_after_startup()
                            && new_path != errors.config().history.path
                        {
                            errors.locked_after_startup(path, val.span());
                            continue;
                        }

                        self.path = new_path;
                    }
                    Value::Nothing { .. } => {
                        if errors.history_locked_after_startup()
                            && errors.config().history.path != HistoryPath::Disabled
                        {
                            errors.locked_after_startup(path, val.span());
                            continue;
                        }

                        self.path = HistoryPath::Disabled;
                    }
                    _ => {
                        errors.type_mismatch(path, Type::custom("string or nothing"), val);
                    }
                },
                "ignore_space_prefixed" => self.ignore_space_prefixed.update(val, path, errors),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;

    fn config_with_history_path(path: HistoryPath) -> Config {
        let mut config = Config::default();
        config.history.path = path;
        config
    }

    #[test]
    fn lock_blocks_changing_to_custom_path() {
        let old = config_with_history_path(HistoryPath::Default);
        let mut new = old.clone();
        let value = Value::test_record(record! {
            "history" => Value::test_record(record! {
                "path" => Value::test_string("/tmp/locked.txt"),
            }),
        });

        let result = new.update_from_value_with_options(&old, &value, true);

        let err = result.expect_err("should fail when locked");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("LockedAfterStartup"),
            "expected LockedAfterStartup error, got: {msg}",
        );
        assert_eq!(new.history.path, HistoryPath::Default);
    }

    #[test]
    fn lock_blocks_disabling_history_at_runtime() {
        let old = config_with_history_path(HistoryPath::Custom("/tmp/h.txt".into()));
        let mut new = old.clone();
        let value = Value::test_record(record! {
            "history" => Value::test_record(record! {
                "path" => Value::nothing(Span::test_data()),
            }),
        });

        let result = new.update_from_value_with_options(&old, &value, true);

        let err = result.expect_err("should fail when locked");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("LockedAfterStartup"),
            "expected LockedAfterStartup error, got: {msg}",
        );
        assert_eq!(new.history.path, HistoryPath::Custom("/tmp/h.txt".into()));
    }

    #[test]
    fn lock_allows_setting_same_value() {
        let old = config_with_history_path(HistoryPath::Custom("/tmp/h.txt".into()));
        let mut new = old.clone();
        let value = Value::test_record(record! {
            "history" => Value::test_record(record! {
                "path" => Value::test_string("/tmp/h.txt"),
            }),
        });

        let result = new.update_from_value_with_options(&old, &value, true);

        assert!(
            result.is_ok(),
            "no-op assignment should succeed: {result:?}"
        );
        assert_eq!(new.history.path, HistoryPath::Custom("/tmp/h.txt".into()));
    }

    #[test]
    fn lock_allows_setting_default_when_already_default() {
        let old = config_with_history_path(HistoryPath::Default);
        let mut new = old.clone();
        let value = Value::test_record(record! {
            "history" => Value::test_record(record! {
                "path" => Value::test_string(""),
            }),
        });

        let result = new.update_from_value_with_options(&old, &value, true);

        assert!(
            result.is_ok(),
            "no-op assignment should succeed: {result:?}"
        );
        assert_eq!(new.history.path, HistoryPath::Default);
    }

    #[test]
    fn unlocked_update_changes_path() {
        let old = config_with_history_path(HistoryPath::Default);
        let mut new = old.clone();
        let value = Value::test_record(record! {
            "history" => Value::test_record(record! {
                "path" => Value::test_string("/tmp/unlocked.txt"),
            }),
        });

        let result = new.update_from_value_with_options(&old, &value, false);

        assert!(result.is_ok(), "unlocked update should succeed: {result:?}");
        assert_eq!(
            new.history.path,
            HistoryPath::Custom("/tmp/unlocked.txt".into())
        );
    }

    #[test]
    fn lock_blocks_changing_max_size() {
        let old = Config::default();
        let original_max_size = old.history.max_size;
        let mut new = old.clone();
        let value = Value::test_record(record! {
            "history" => Value::test_record(record! {
                "max_size" => Value::test_int(original_max_size + 1),
            }),
        });

        let result = new.update_from_value_with_options(&old, &value, true);

        let err = result.expect_err("should fail when locked");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("LockedAfterStartup"),
            "expected LockedAfterStartup error, got: {msg}",
        );
        assert_eq!(new.history.max_size, original_max_size);
    }

    #[test]
    fn lock_allows_setting_same_max_size() {
        let old = Config::default();
        let original_max_size = old.history.max_size;
        let mut new = old.clone();
        let value = Value::test_record(record! {
            "history" => Value::test_record(record! {
                "max_size" => Value::test_int(original_max_size),
            }),
        });

        let result = new.update_from_value_with_options(&old, &value, true);

        assert!(
            result.is_ok(),
            "no-op assignment should succeed: {result:?}"
        );
        assert_eq!(new.history.max_size, original_max_size);
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn lock_blocks_changing_file_format() {
        let old = Config::default();
        let original_file_format = old.history.file_format;
        let (_other_format, other_format_str) = match original_file_format {
            HistoryFileFormat::Plaintext => (HistoryFileFormat::Sqlite, "sqlite"),
            HistoryFileFormat::Sqlite => (HistoryFileFormat::Plaintext, "plaintext"),
        };
        let mut new = old.clone();
        let value = Value::test_record(record! {
            "history" => Value::test_record(record! {
                "file_format" => Value::test_string(other_format_str),
            }),
        });

        let result = new.update_from_value_with_options(&old, &value, true);

        let err = result.expect_err("should fail when locked");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("LockedAfterStartup"),
            "expected LockedAfterStartup error, got: {msg}",
        );
        assert_eq!(new.history.file_format, original_file_format);
    }

    #[test]
    fn lock_blocks_changing_isolation() {
        let old = Config::default();
        let original_isolation = old.history.isolation;
        let mut new = old.clone();
        let value = Value::test_record(record! {
            "history" => Value::test_record(record! {
                "isolation" => Value::test_bool(!original_isolation),
            }),
        });

        let result = new.update_from_value_with_options(&old, &value, true);

        let err = result.expect_err("should fail when locked");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("LockedAfterStartup"),
            "expected LockedAfterStartup error, got: {msg}",
        );
        assert_eq!(new.history.isolation, original_isolation);
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn unlocked_update_changes_max_size_file_format_isolation() {
        let old = Config::default();
        let mut new = old.clone();
        let new_max_size = old.history.max_size + 1;
        let (new_file_format, new_file_format_str) = match old.history.file_format {
            HistoryFileFormat::Plaintext => (HistoryFileFormat::Sqlite, "sqlite"),
            HistoryFileFormat::Sqlite => (HistoryFileFormat::Plaintext, "plaintext"),
        };
        let new_isolation = !old.history.isolation;
        let value = Value::test_record(record! {
            "history" => Value::test_record(record! {
                "max_size" => Value::test_int(new_max_size),
                "file_format" => Value::test_string(new_file_format_str),
                "isolation" => Value::test_bool(new_isolation),
            }),
        });

        let result = new.update_from_value_with_options(&old, &value, false);

        assert!(result.is_ok(), "unlocked update should succeed: {result:?}");
        assert_eq!(new.history.max_size, new_max_size);
        assert_eq!(new.history.file_format, new_file_format);
        assert_eq!(new.history.isolation, new_isolation);
    }
}
