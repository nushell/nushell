//! Functionality and types shared between the plugin and the engine, other than protocol types.
//!
//! If you are writing a plugin, you probably don't need this crate. We will make fewer guarantees
//! for the stability of the interface of this crate than for `nu_plugin`.

pub mod util;

mod communication_mode;
mod interface;
mod serializers;

pub use communication_mode::{
    ClientCommunicationIo, CommunicationMode, PreparedServerCommunication, ServerCommunicationIo,
};
pub use interface::{
    Interface, InterfaceManager, PipelineDataWriter, PluginRead, PluginWrite,
    stream::{FromShellError, StreamManager, StreamManagerHandle, StreamReader, StreamWriter},
};
pub use serializers::{
    Encoder, EncodingType, PluginEncoder, json::JsonSerializer, msgpack::MsgPackSerializer,
};

#[doc(hidden)]
pub use interface::test_util as interface_test_util;
