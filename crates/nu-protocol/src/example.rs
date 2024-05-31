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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginExample {
    pub example: String,
    pub description: String,
    pub result: Option<Value>,
}

#[cfg(feature = "plugin")]
impl From<Example<'_>> for PluginExample {
    fn from(value: Example) -> Self {
        PluginExample {
            example: value.example.into(),
            description: value.description.into(),
            result: value.result,
        }
    }
}
