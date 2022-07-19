mod plugin;
mod protocol;
mod serializers;

#[allow(dead_code)]
mod plugin_capnp;

pub use plugin::{get_signature, plugin_data::PluginData, serve_plugin, Plugin, PluginDeclaration};
pub use protocol::{EvaluatedCall, LabeledError};
pub use serializers::{capnp::CapnpSerializer, json::JsonSerializer, EncodingType};
