use crate::config::{last_modified, read, Conf, Status};
use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_protocol::Value;
use nu_source::Tag;
use nu_test_support::NATIVE_PATH_ENV_VAR;
use std::{fmt::Debug, path::PathBuf};

use super::write;

#[derive(Debug, Clone, Default)]
pub struct NuConfig {
    pub vars: IndexMap<String, Value>,
    pub file_path: PathBuf,
    pub modified_at: Status,
}

impl Conf for NuConfig {
    fn is_modified(&self) -> Result<bool, Box<dyn std::error::Error>> {
        self.is_modified()
    }

    fn var(&self, key: &str) -> Option<Value> {
        self.var(key)
    }

    fn env(&self) -> Option<Value> {
        self.env()
    }

    fn path(&self) -> Result<Option<Vec<PathBuf>>, ShellError> {
        self.path()
    }

    fn reload(&mut self) {
        if let Ok(variables) = read(Tag::unknown(), &Some(self.file_path.clone())) {
            self.vars = variables;

            self.modified_at = if let Ok(status) = last_modified(&Some(self.file_path.clone())) {
                status
            } else {
                Status::Unavailable
            };
        }
    }

    fn clone_box(&self) -> Box<dyn Conf> {
        Box::new(self.clone())
    }
}

impl NuConfig {
    pub fn load(cfg_file_path: Option<PathBuf>) -> Result<NuConfig, ShellError> {
        let vars = read(Tag::unknown(), &cfg_file_path)?;
        let modified_at = NuConfig::get_last_modified(&cfg_file_path);
        let file_path = if let Some(file_path) = cfg_file_path {
            file_path
        } else {
            crate::config::default_path()?
        };

        Ok(NuConfig {
            vars,
            file_path,
            modified_at,
        })
    }

    /// Writes self.values under self.file_path
    pub fn write(&self) -> Result<(), ShellError> {
        write(&self.vars, &Some(self.file_path.clone()))
    }

    pub fn new() -> NuConfig {
        let vars = if let Ok(variables) = read(Tag::unknown(), &None) {
            variables
        } else {
            IndexMap::default()
        };
        let path = if let Ok(path) = crate::config::default_path() {
            path
        } else {
            PathBuf::new()
        };

        NuConfig {
            vars,
            modified_at: NuConfig::get_last_modified(&None),
            file_path: path,
        }
    }

    pub fn get_last_modified(config_file: &Option<std::path::PathBuf>) -> Status {
        if let Ok(status) = last_modified(config_file) {
            status
        } else {
            Status::Unavailable
        }
    }

    pub fn is_modified(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let modified_at = &self.modified_at;

        Ok(match (NuConfig::get_last_modified(&None), modified_at) {
            (Status::LastModified(left), Status::LastModified(right)) => {
                let left = left.duration_since(std::time::UNIX_EPOCH)?;
                let right = (*right).duration_since(std::time::UNIX_EPOCH)?;

                left != right
            }
            (_, _) => false,
        })
    }

    pub fn var(&self, key: &str) -> Option<Value> {
        let vars = &self.vars;

        if let Some(value) = vars.get(key) {
            return Some(value.clone());
        }

        None
    }

    /// Return environment variables as map
    pub fn env_map(&self) -> IndexMap<String, String> {
        let mut result = IndexMap::new();
        if let Some(variables) = self.env() {
            for var in variables.row_entries() {
                if let Ok(value) = var.1.as_string() {
                    result.insert(var.0.clone(), value);
                }
            }
        }
        result
    }

    pub fn env(&self) -> Option<Value> {
        let vars = &self.vars;

        if let Some(env_vars) = vars.get("env") {
            return Some(env_vars.clone());
        }

        None
    }

    pub fn path(&self) -> Result<Option<Vec<PathBuf>>, ShellError> {
        let vars = &self.vars;

        if let Some(path) = vars.get("path").or_else(|| vars.get(NATIVE_PATH_ENV_VAR)) {
            path
                .table_entries()
                .map(|p| {
                    p.as_string().map(PathBuf::from).map_err(|_| {
                        ShellError::untagged_runtime_error("Could not format path entry as string!\nPath entry from config won't be added")
                    })
                })
            .collect::<Result<Vec<PathBuf>, ShellError>>().map(Some)
        } else {
            Ok(None)
        }
    }

    fn load_scripts_if_present(&self, scripts_name: &str) -> Result<Vec<String>, ShellError> {
        if let Some(array) = self.var(scripts_name) {
            if !array.is_table() {
                Err(ShellError::untagged_runtime_error(format!(
                    "expected an array of strings as {} commands",
                    scripts_name
                )))
            } else {
                array.table_entries().map(Value::as_string).collect()
            }
        } else {
            Ok(vec![])
        }
    }

    pub fn exit_scripts(&self) -> Result<Vec<String>, ShellError> {
        self.load_scripts_if_present("on_exit")
    }

    pub fn startup_scripts(&self) -> Result<Vec<String>, ShellError> {
        self.load_scripts_if_present("startup")
    }
}
