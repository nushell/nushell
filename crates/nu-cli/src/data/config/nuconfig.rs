use crate::data::config::{read, Conf};
use indexmap::IndexMap;
use nu_protocol::Value;
use nu_source::Tag;
use parking_lot::Mutex;
use std::fmt::Debug;
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
pub struct NuConfig {
    pub vars: Arc<Mutex<IndexMap<String, Value>>>,
}

impl Conf for NuConfig {
    fn env(&self) -> Option<Value> {
        self.env()
    }

    fn path(&self) -> Option<Value> {
        self.path()
    }

    fn reload(&self) {
        let mut vars = self.vars.lock();

        if let Ok(variables) = read(Tag::unknown(), &None) {
            vars.extend(variables);
        }
    }
}

impl NuConfig {
    pub fn new() -> NuConfig {
        let vars = if let Ok(variables) = read(Tag::unknown(), &None) {
            variables
        } else {
            IndexMap::default()
        };

        NuConfig {
            vars: Arc::new(Mutex::new(vars)),
        }
    }

    pub fn env(&self) -> Option<Value> {
        let vars = self.vars.lock();

        if let Some(env_vars) = vars.get("env") {
            return Some(env_vars.clone());
        }

        None
    }

    pub fn path(&self) -> Option<Value> {
        let vars = self.vars.lock();

        if let Some(env_vars) = vars.get("path") {
            return Some(env_vars.clone());
        }

        None
    }
}
