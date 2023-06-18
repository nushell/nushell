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

#[derive(Clone, Debug)]
pub enum EncodingType {
    Json,
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
