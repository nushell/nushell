mod plugin;
mod protocol;
mod serializers;

pub use plugin::{get_signature, serve_plugin, Plugin, PluginDeclaration};
pub use protocol::{EvaluatedCall, LabeledError, PluginData, PluginResponse};
pub use serializers::{json::JsonSerializer, msgpack::MsgPackSerializer, EncodingType};
