use crate::Value;
#[cfg(feature = "plugin")]
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct Example<'a> {
    pub example: &'a str,
    pub description: &'a str,
    pub result: Option<Value>,
}

// PluginExample is somehow like struct `Example`, but it owned a String for `example`
// and `description` fields, because these information is fetched from plugin, a third party
// binary, nushell have no way to construct it directly.
#[cfg(feature = "plugin")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginExample {
    pub example: String,
    pub description: String,
    pub result: Option<Value>,
}
