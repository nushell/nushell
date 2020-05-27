use crate::value::Value;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// An evaluation scope. Scopes map variable names to Values and aid in evaluating blocks and expressions.
/// Additionally, holds the value for the special $it variable, a variable used to refer to the value passing
/// through the pipeline at that moment
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Scope {
    pub it: Value,
    pub vars: IndexMap<String, Value>,
    pub env: IndexMap<String, String>,
}

impl Scope {
    /// Create an empty scope
    pub fn new() -> Scope {
        Scope {
            it: Value::nothing(),
            vars: IndexMap::new(),
            env: IndexMap::new(),
        }
    }
}

impl Default for Scope {
    fn default() -> Scope {
        Scope::new()
    }
}
