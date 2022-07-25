use nu_protocol::Span;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct PluginData {
    pub data: Vec<u8>,
    pub span: Span,
}
