use std::path::PathBuf;

use nu_protocol::{CustomValue, Value};
use serde::Serialize;

use crate::EncodingType;

/// An opaque container for a custom value that is handled fully by a plugin
///
/// This is constructed by the main nushell engine when it receives [`PluginResponse::PluginData`]
/// it stores that data as well as metadata related to the plugin to be able to call the plugin
/// later.
/// Since the data in it is opaque to the engine, there are only two final destinations for it:
/// either it will be sent back to the plugin that generated it across a pipeline, or it will be
/// sent to the plugin with a request to collapse it into a base value
#[derive(Clone, Debug, Serialize)]
pub struct PluginCustomValue {
    pub data: serde_json::Value,
    pub filename: PathBuf,

    // PluginCustomValue must implement Serialize because all CustomValues must implement Serialize
    // However, the main place where values are serialized and deserialized is when they are being
    // sent between plugins and nushell's main engine. PluginCustomValue is never meant to be sent
    // between that boundary
    #[serde(skip)]
    pub shell: Option<PathBuf>,
    #[serde(skip)]
    pub encoding: EncodingType,
    #[serde(skip)]
    pub source: String,
}

impl CustomValue for PluginCustomValue {
    fn clone_value(&self, span: nu_protocol::Span) -> nu_protocol::Value {
        Value::CustomValue {
            val: Box::new(self.clone()),
            span,
        }
    }

    fn value_string(&self) -> String {
        self.typetag_name().to_string()
    }

    fn to_base_value(
        &self,
        _span: nu_protocol::Span,
    ) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
        todo!()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn typetag_name(&self) -> &'static str {
        // TODO: Is this a good idea? I'd love to be able to get the name of the data type itself
        // but I don't think there's a way to get a &'static str without leaking which isn't ideal
        "PluginCustomValue"
    }

    fn typetag_deserialize(&self) {
        unimplemented!("typetag_deserialize")
    }
}
