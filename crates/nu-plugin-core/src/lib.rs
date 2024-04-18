//! Functionality and types shared between the plugin and the engine, other than protocol types.

pub mod util;

mod communication_mode;
mod interface;
mod serializers;

pub use communication_mode::{
    ClientCommunicationIo, CommunicationMode, PreparedServerCommunication, ServerCommunicationIo,
};
pub use interface::{
    stream::{StreamManager, StreamManagerHandle, StreamReader, StreamWriter},
    Interface, InterfaceManager, PipelineDataWriter, PluginRead, PluginWrite,
};
pub use serializers::{
    json::JsonSerializer, msgpack::MsgPackSerializer, Encoder, EncodingType, PluginEncoder,
};

#[doc(hidden)]
pub use interface::test_util as interface_test_util;
