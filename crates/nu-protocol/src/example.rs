use crate::Value;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct Example<'a> {
    pub example: &'a str,
    pub description: &'a str,
    pub result: Option<Value>,
}

#[cfg(feature = "plugin")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginExample {
    pub example: String,
    pub description: String,
    pub result: Option<Value>,
}
