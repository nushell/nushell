<<<<<<< HEAD
pub mod jsonrpc;
mod plugin;

pub mod test_helpers;

pub use crate::plugin::{serve_plugin, Plugin};
=======
mod plugin;
mod protocol;
mod serializers;

#[allow(dead_code)]
mod plugin_capnp;

pub use plugin::{get_signature, serve_plugin, Plugin, PluginDeclaration};
pub use protocol::{EvaluatedCall, LabeledError};
pub use serializers::{capnp::CapnpSerializer, json::JsonSerializer, EncodingType};
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
