#[cfg(feature="nu-internal")]
use {
    nu_protocol::ShellError,
    crate::{
        plugin::PluginEncoder,
        protocol::PluginResponse,
    },
};

pub(crate) mod json;
pub(crate) mod msgpack;

#[cfg(feature="nu-internal")]
use self::{json::JsonSerializer, msgpack::MsgPackSerializer};

/// Indicates what encoding type should be used when serving the [Plugin](crate::Plugin)
/// 
/// An `EncodingType` is passed to [`serve_plugin`](crate::serve_plugin) to indicate
/// how data should be serialized when passed between the Plugin executable and 
/// Nushell.
#[derive(Clone, Debug)]
pub enum EncodingType {
    /// Use the [JSON](https://www.json.org/) text serialization format
    Json,
    /// Use the [MsgPack](https://msgpack.org/index.html) binary serialization format
    MsgPack,
}

impl EncodingType {
    pub(crate) fn try_from_bytes(bytes: &[u8]) -> Option<Self> {
        match bytes {
            b"json" => Some(Self::Json),
            b"msgpack" => Some(Self::MsgPack),
            _ => None,
        }
    }
}

#[cfg(feature = "nu-internal")]
impl EncodingType {
    /// Internal method used by Nushell
    /// 
    /// Hidden behind the `nu-internal` feature.
    pub fn encode_call(
        &self,
        plugin_call: &crate::protocol::PluginCall,
        writer: &mut impl std::io::Write,
    ) -> Result<(), nu_protocol::ShellError> {
        match self {
            EncodingType::Json => JsonSerializer::encode_call(plugin_call, writer),
            EncodingType::MsgPack => MsgPackSerializer::encode_call(plugin_call, writer),
        }
    }

    /// Internal method used by Nushell
    /// 
    /// Hidden behind the `nu-internal` feature.
    pub fn decode_response(
        &self,
        reader: &mut impl std::io::BufRead,
    ) -> Result<PluginResponse, ShellError> {
        match self {
            EncodingType::Json => JsonSerializer::decode_response(reader),
            EncodingType::MsgPack => MsgPackSerializer::decode_response(reader),
        }
    }
}
