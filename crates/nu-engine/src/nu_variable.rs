use crate::scope::create_scope;
use core::fmt;
use nu_protocol::{
    engine::{EngineState, Stack},
    HistoryFileFormat, LazyRecord, ShellError, Span, Value,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use sysinfo::SystemExt;

// a LazyRecord for the special $nu variable
// $nu used to be a plain old Record, but LazyRecord lets us load different fields/columns lazily. This is important for performance;
// collecting all the information in $nu is expensive and unnecessary if  you just want a subset of the data
#[derive(Serialize, Deserialize)]
pub struct NuVariable {
    #[serde(skip)]
    pub engine_state: EngineState,
    #[serde(skip)]
    pub stack: Stack,
    pub span: Span,
}

impl LazyRecord for NuVariable {
    fn value_string(&self) -> String {
        "$nu".to_string()
    }

    fn get_column_map(&self) -> HashMap<String, Box<dyn Fn() -> Result<Value, ShellError> + '_>> {
        let mut map: HashMap<_, Box<dyn Fn() -> Result<Value, ShellError>>> = HashMap::new();

        if let Some(path) = self.engine_state.get_config_path("config-path") {
            map.insert(
                "config-path".to_string(),
                Box::new(move || {
                    Ok(Value::String {
                        val: path.to_string_lossy().to_string(),
                        span: self.span(),
                    })
                }),
            );
        }

        if let Some(path) = self.engine_state.get_config_path("env-path") {
            map.insert(
                "env-path".to_string(),
                Box::new(move || {
                    Ok(Value::String {
                        val: path.to_string_lossy().to_string(),
                        span: self.span(),
                    })
                }),
            );
        }

        if let Some(mut config_path) = nu_path::config_dir() {
            config_path.push("nushell");
            let mut env_config_path = config_path.clone();
            let mut loginshell_path = config_path.clone();
            let mut history_path = config_path.clone();

            match self.engine_state.config.history_file_format {
                HistoryFileFormat::Sqlite => {
                    history_path.push("history.sqlite3");
                }
                HistoryFileFormat::PlainText => {
                    history_path.push("history.txt");
                }
            }
            // let mut history_path = config_files::get_history_path(); // todo: this should use the get_history_path method but idk where to put that function

            map.insert(
                "history-path".to_string(),
                Box::new(move || {
                    Ok(Value::String {
                        val: history_path.to_string_lossy().to_string(),
                        span: self.span(),
                    })
                }),
            );

            if !map.contains_key("config-path") {
                config_path.push("config.nu");
                map.insert(
                    "config-path".to_string(),
                    Box::new(move || {
                        Ok(Value::String {
                            val: config_path.to_string_lossy().to_string(),
                            span: self.span(),
                        })
                    }),
                );
            }

            if !map.contains_key("env-path") {
                env_config_path.push("env.nu");
                map.insert(
                    "env-path".to_string(),
                    Box::new(move || {
                        Ok(Value::String {
                            val: env_config_path.to_string_lossy().to_string(),
                            span: self.span(),
                        })
                    }),
                );
            }

            loginshell_path.push("login.nu");

            map.insert(
                "loginshell-path".to_string(),
                Box::new(move || {
                    Ok(Value::String {
                        val: loginshell_path.to_string_lossy().to_string(),
                        span: self.span(),
                    })
                }),
            );
        }

        #[cfg(feature = "plugin")]
        if let Some(path) = &self.engine_state.plugin_signatures {
            if let Some(path_str) = path.to_str() {
                map.insert(
                    "plugin-path".to_string(),
                    Box::new(move || {
                        Ok(Value::String {
                            val: path_str.into(),
                            span: self.span(),
                        })
                    }),
                );
            }
        }

        map.insert(
            "scope".to_string(),
            Box::new(move || Ok(create_scope(&self.engine_state, &self.stack, self.span())?)),
        );

        if let Some(home_path) = nu_path::home_dir() {
            map.insert(
                "home-path".into(),
                Box::new(move || {
                    Ok(Value::String {
                        val: home_path.to_string_lossy().into(),
                        span: self.span(),
                    })
                }),
            );
        }

        map.insert(
            "temp-path".into(),
            Box::new(move || {
                let temp_path = std::env::temp_dir();
                Ok(Value::String {
                    val: temp_path.to_string_lossy().into(),
                    span: self.span(),
                })
            }),
        );

        map.insert(
            "pid".into(),
            Box::new(move || Ok(Value::int(std::process::id().into(), self.span()))),
        );

        map.insert(
            "os-info".into(),
            Box::new(move || {
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
            }),
        );

        map
    }

    fn typetag_name(&self) -> &'static str {
        todo!()
    }

    fn typetag_deserialize(&self) {
        todo!()
    }

    fn span(&self) -> Span {
        self.span
    }
}

// manually implemented so we can skip engine_state which doesn't implement Debug
// FIXME: find a better way
impl fmt::Debug for NuVariable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NuVariable").finish()
    }
}
