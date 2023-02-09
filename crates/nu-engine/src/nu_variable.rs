use crate::scope::create_scope;
use core::fmt;
use nu_protocol::{
    engine::{EngineState, Stack},
    LazyRecord, ShellError, Span, Value,
};
use serde::{Deserialize, Serialize};
use sysinfo::SystemExt;

// NuVariable: a LazyRecord for the special $nu variable
// $nu used to be a plain old Record, but LazyRecord lets us load different fields/columns lazily. This is important for performance;
// collecting all the information in $nu is expensive and unnecessary if you just want a subset of the data

// Note: NuVariable is not meaningfully serializable, this #[derive] is a lie to satisfy the type checker.
// Make sure to collect() the record before serializing it
#[derive(Serialize, Deserialize)]
pub struct NuVariable {
    #[serde(skip)]
    pub engine_state: EngineState,
    #[serde(skip)]
    pub stack: Stack,
    pub span: Span,
}

impl LazyRecord for NuVariable {
    fn column_names(&self) -> Vec<&'static str> {
        let mut cols = vec!["config-path", "env-path", "history-path", "loginshell-path"];

        #[cfg(feature = "plugin")]
        if self.engine_state.plugin_signatures.is_some() {
            cols.push("plugin-path");
        }

        cols.push("scope");
        cols.push("home-path");
        cols.push("temp-path");
        cols.push("pid");
        cols.push("os-info");

        cols
    }

    fn get_column_value(&self, column: &str) -> Result<Value, ShellError> {
        let err = |message: &str| -> Result<Value, ShellError> {
            Err(ShellError::LazyRecordAccessFailed {
                message: message.into(),
                column_name: column.to_string(),
                span: self.span,
            })
        };

        match column {
            "config-path" => {
                if let Some(path) = self.engine_state.get_config_path("config-path") {
                    Ok(Value::String {
                        val: path.to_string_lossy().to_string(),
                        span: self.span,
                    })
                } else if let Some(mut path) = nu_path::config_dir() {
                    path.push("nushell");
                    path.push("config.nu");
                    Ok(Value::String {
                        val: path.to_string_lossy().to_string(),
                        span: self.span,
                    })
                } else {
                    err("Could not get config directory")
                }
            }
            "env-path" => {
                if let Some(path) = self.engine_state.get_config_path("env-path") {
                    Ok(Value::String {
                        val: path.to_string_lossy().to_string(),
                        span: self.span,
                    })
                } else if let Some(mut path) = nu_path::config_dir() {
                    path.push("nushell");
                    path.push("env.nu");
                    Ok(Value::String {
                        val: path.to_string_lossy().to_string(),
                        span: self.span,
                    })
                } else {
                    err("Could not get config directory")
                }
            }
            "history-path" => {
                if let Some(mut path) = nu_path::config_dir() {
                    path.push("nushell");
                    match self.engine_state.config.history_file_format {
                        nu_protocol::HistoryFileFormat::Sqlite => {
                            path.push("history.sqlite3");
                        }
                        nu_protocol::HistoryFileFormat::PlainText => {
                            path.push("history.txt");
                        }
                    }
                    Ok(Value::String {
                        val: path.to_string_lossy().to_string(),
                        span: self.span,
                    })
                } else {
                    err("Could not get config directory")
                }
            }
            "loginshell-path" => {
                if let Some(mut path) = nu_path::config_dir() {
                    path.push("nushell");
                    path.push("login.nu");
                    Ok(Value::String {
                        val: path.to_string_lossy().to_string(),
                        span: self.span,
                    })
                } else {
                    err("Could not get config directory")
                }
            }
            "plugin-path" => {
                #[cfg(feature = "plugin")]
                {
                    if let Some(path) = &self.engine_state.plugin_signatures {
                        Ok(Value::String {
                            val: path.to_string_lossy().to_string(),
                            span: self.span,
                        })
                    } else {
                        err("Could not get plugin signature location")
                    }
                }

                #[cfg(not(feature = "plugin"))]
                {
                    err("Plugin feature not enabled")
                }
            }
            "scope" => Ok(create_scope(&self.engine_state, &self.stack, self.span())?),
            "home-path" => {
                if let Some(home_path) = nu_path::home_dir() {
                    Ok(Value::String {
                        val: home_path.to_string_lossy().into(),
                        span: self.span(),
                    })
                } else {
                    err("Could not get home path")
                }
            }
            "temp-path" => {
                let temp_path = std::env::temp_dir();
                Ok(Value::String {
                    val: temp_path.to_string_lossy().into(),
                    span: self.span(),
                })
            }
            "pid" => Ok(Value::int(std::process::id().into(), self.span())),
            "os-info" => {
                let sys = sysinfo::System::new();
                let ver = match sys.kernel_version() {
                    Some(v) => v,
                    None => "unknown".into(),
                };

                let os_record = Value::Record {
                    cols: vec![
                        "name".into(),
                        "arch".into(),
                        "family".into(),
                        "kernel_version".into(),
                    ],
                    vals: vec![
                        Value::string(std::env::consts::OS, self.span()),
                        Value::string(std::env::consts::ARCH, self.span()),
                        Value::string(std::env::consts::FAMILY, self.span()),
                        Value::string(ver, self.span()),
                    ],
                    span: self.span(),
                };

                Ok(os_record)
            }
            _ => err(&format!("Could not find column '{column}'")),
        }
    }

    fn span(&self) -> Span {
        self.span
    }

    fn typetag_name(&self) -> &'static str {
        "nu_variable"
    }

    fn typetag_deserialize(&self) {
        unimplemented!("typetag_deserialize")
    }
}

// manually implemented so we can skip engine_state which doesn't implement Debug
// FIXME: find a better way
impl fmt::Debug for NuVariable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NuVariable").finish()
    }
}
