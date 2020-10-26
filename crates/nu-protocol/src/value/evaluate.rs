use crate::value::Value;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::Arc;

/// An evaluation scope. Scopes map variable names to Values and aid in evaluating blocks and expressions.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Scope {
    vars: IndexMap<String, Value>,
    env: IndexMap<String, String>,
    parent: Option<Arc<Scope>>,
}

impl Scope {
    pub fn vars(&self) -> IndexMap<String, Value> {
        //FIXME: should this be an interator?

        let mut output = IndexMap::new();

        for v in &self.vars {
            output.insert(v.0.clone(), v.1.clone());
        }

        if let Some(parent) = &self.parent {
            for v in parent.vars() {
                if !output.contains_key(&v.0) {
                    output.insert(v.0.clone(), v.1.clone());
                }
            }
        }

        output
    }

    pub fn env(&self) -> IndexMap<String, String> {
        //FIXME: should this be an interator?

        let mut output = IndexMap::new();

        for v in &self.env {
            output.insert(v.0.clone(), v.1.clone());
        }

        if let Some(parent) = &self.parent {
            for v in parent.env() {
                if !output.contains_key(&v.0) {
                    output.insert(v.0.clone(), v.1.clone());
                }
            }
        }

        output
    }

    pub fn var(&self, name: &str) -> Option<Value> {
        if let Some(value) = self.vars().get(name) {
            Some(value.clone())
        } else {
            None
        }
    }

    pub fn from_env(env: IndexMap<String, String>) -> Arc<Scope> {
        Arc::new(Scope {
            vars: IndexMap::new(),
            env,
            parent: None,
        })
    }

    pub fn append_var(this: Arc<Self>, name: impl Into<String>, value: Value) -> Arc<Scope> {
        let mut vars = IndexMap::new();
        vars.insert(name.into(), value);
        Arc::new(Scope {
            vars,
            env: IndexMap::new(),
            parent: Some(this),
        })
    }

    pub fn append_vars(this: Arc<Self>, vars: IndexMap<String, Value>) -> Arc<Scope> {
        Arc::new(Scope {
            vars,
            env: IndexMap::new(),
            parent: Some(this),
        })
    }

    pub fn append_env(this: Arc<Self>, env: IndexMap<String, String>) -> Arc<Scope> {
        Arc::new(Scope {
            vars: IndexMap::new(),
            env,
            parent: Some(this),
        })
    }

    /// Create an empty scope
    pub fn create() -> Arc<Scope> {
        Arc::new(Scope {
            vars: IndexMap::new(),
            env: IndexMap::new(),
            parent: None,
        })
    }
}
