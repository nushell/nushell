mod plugin;
mod protocol;
mod serializers;

#[allow(dead_code)]
mod plugin_capnp;

pub use plugin::{get_signature, serve_plugin, Plugin, PluginDeclaration};
pub use protocol::{EvaluatedCall, LabeledError};
pub use serializers::{capnp::CapnpSerializer, json::JsonSerializer, EncodingType};
