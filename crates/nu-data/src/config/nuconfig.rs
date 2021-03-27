use crate::config::{last_modified, read, Conf, Status};
use indexmap::IndexMap;
use nu_protocol::Value;
use nu_source::Tag;
use std::any::Any;
use std::fmt::Debug;
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct NuConfig {
    pub source_file: Option<std::path::PathBuf>,
    pub vars: IndexMap<String, Value>,
    pub modified_at: Status,
}

impl Conf for NuConfig {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn is_modified(&self) -> Result<bool, Box<dyn std::error::Error>> {
        self.is_modified()
    }

    fn var(&self, key: &str) -> Option<Value> {
        self.var(key)
    }

    fn env(&self) -> Option<Value> {
        self.env()
    }

    fn path(&self) -> Option<Value> {
        self.path()
    }

    fn reload(&mut self) {
        let vars = &mut self.vars;

        if let Ok(variables) = read(Tag::unknown(), &self.source_file) {
            vars.extend(variables);

            self.modified_at = if let Ok(status) = last_modified(&None) {
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
    pub fn with(config_file: Option<std::ffi::OsString>) -> NuConfig {
        match &config_file {
            None => NuConfig::new(),
            Some(_) => {
                let source_file = config_file.map(std::path::PathBuf::from);

                let vars = if let Ok(variables) = read(Tag::unknown(), &source_file) {
                    variables
                } else {
                    IndexMap::default()
                };

                NuConfig {
                    source_file: source_file.clone(),
                    vars,
                    modified_at: NuConfig::get_last_modified(&source_file),
                }
            }
        }
    }

    pub fn new() -> NuConfig {
        let vars = if let Ok(variables) = read(Tag::unknown(), &None) {
            variables
        } else {
            IndexMap::default()
        };

        NuConfig {
            source_file: None,
            vars,
            modified_at: NuConfig::get_last_modified(&None),
        }
    }

    pub fn history_path(&self) -> PathBuf {
        super::path::history(self)
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

        Ok(
            match (NuConfig::get_last_modified(&self.source_file), modified_at) {
                (Status::LastModified(left), Status::LastModified(right)) => {
                    let left = left.duration_since(std::time::UNIX_EPOCH)?;
                    let right = (*right).duration_since(std::time::UNIX_EPOCH)?;

                    left != right
                }
                (_, _) => false,
            },
        )
    }

    pub fn var(&self, key: &str) -> Option<Value> {
        let vars = &self.vars;

        if let Some(value) = vars.get(key) {
            return Some(value.clone());
        }

        None
    }

    pub fn env(&self) -> Option<Value> {
        let vars = &self.vars;

        if let Some(env_vars) = vars.get("env") {
            return Some(env_vars.clone());
        }

        None
    }

    pub fn path(&self) -> Option<Value> {
        let vars = &self.vars;

        if let Some(env_vars) = vars.get("path") {
            return Some(env_vars.clone());
        }

        if let Some(env_vars) = vars.get("PATH") {
            return Some(env_vars.clone());
        }

        None
    }
}
