use crate::{
    plugin::PluginEncoder,
    protocol::{PluginCall, PluginResponse},
};
use nu_protocol::ShellError;

pub mod json;
pub mod msgpack;

#[derive(Clone, Debug)]
pub enum EncodingType {
    Json(json::JsonSerializer),
    MsgPack(msgpack::MsgPackSerializer),
}

impl EncodingType {
    pub fn try_from_bytes(bytes: &[u8]) -> Option<Self> {
        match bytes {
            b"json" => Some(Self::Json(json::JsonSerializer {})),
            b"msgpack" => Some(Self::MsgPack(msgpack::MsgPackSerializer {})),
            _ => None,
        }
    }

    pub fn encode_call(
        &self,
        plugin_call: &PluginCall,
        writer: &mut impl std::io::Write,
    ) -> Result<(), ShellError> {
        match self {
            EncodingType::Json(encoder) => encoder.encode_call(plugin_call, writer),
            EncodingType::MsgPack(encoder) => encoder.encode_call(plugin_call, writer),
        }
    }

    pub fn decode_call(
        &self,
        reader: &mut impl std::io::BufRead,
    ) -> Result<PluginCall, ShellError> {
        match self {
            EncodingType::Json(encoder) => encoder.decode_call(reader),
            EncodingType::MsgPack(encoder) => encoder.decode_call(reader),
        }
    }

    pub fn encode_response(
        &self,
        plugin_response: &PluginResponse,
        writer: &mut impl std::io::Write,
    ) -> Result<(), ShellError> {
        match self {
            EncodingType::Json(encoder) => encoder.encode_response(plugin_response, writer),
            EncodingType::MsgPack(encoder) => encoder.encode_response(plugin_response, writer),
        }
    }

    pub fn decode_response(
        &self,
        reader: &mut impl std::io::BufRead,
    ) -> Result<PluginResponse, ShellError> {
        match self {
            EncodingType::Json(encoder) => encoder.decode_response(reader),
            EncodingType::MsgPack(encoder) => encoder.decode_response(reader),
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            Self::Json(_) => "json",
            Self::MsgPack(_) => "msgpack",
        }
    }
}
