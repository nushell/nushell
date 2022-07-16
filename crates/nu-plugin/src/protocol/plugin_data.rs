use nu_protocol::Span;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct PluginData {
    pub data: serde_json::Value,
    pub span: Span,
}
