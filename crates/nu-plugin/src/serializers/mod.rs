use nu_protocol::ShellError;

use crate::{
    plugin::PluginEncoder,
    protocol::{PluginCall, PluginResponse},
};

pub mod capnp;
pub mod json;

#[derive(Clone)]
pub enum EncodingType {
    Capnp(capnp::CapnpSerializer),
    Json(json::JsonSerializer),
}

impl EncodingType {
    pub fn try_from_bytes(bytes: &[u8]) -> Option<Self> {
        match bytes {
            b"capnp" => Some(Self::Capnp(capnp::CapnpSerializer {})),
            b"json" => Some(Self::Json(json::JsonSerializer {})),
            _ => None,
        }
    }

    pub fn encode_call(
        &self,
        plugin_call: &PluginCall,
        writer: &mut impl std::io::Write,
    ) -> Result<(), ShellError> {
        match self {
            EncodingType::Capnp(encoder) => encoder.encode_call(plugin_call, writer),
            EncodingType::Json(encoder) => encoder.encode_call(plugin_call, writer),
        }
    }

    pub fn decode_call(
        &self,
        reader: &mut impl std::io::BufRead,
    ) -> Result<PluginCall, ShellError> {
        match self {
            EncodingType::Capnp(encoder) => encoder.decode_call(reader),
            EncodingType::Json(encoder) => encoder.decode_call(reader),
        }
    }

    pub fn encode_response(
        &self,
        plugin_response: &PluginResponse,
        writer: &mut impl std::io::Write,
    ) -> Result<(), ShellError> {
        match self {
            EncodingType::Capnp(encoder) => encoder.encode_response(plugin_response, writer),
            EncodingType::Json(encoder) => encoder.encode_response(plugin_response, writer),
        }
    }

    pub fn decode_response(
        &self,
        reader: &mut impl std::io::BufRead,
    ) -> Result<PluginResponse, ShellError> {
        match self {
            EncodingType::Capnp(encoder) => encoder.decode_response(reader),
            EncodingType::Json(encoder) => encoder.decode_response(reader),
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            Self::Capnp(_) => "capnp",
            Self::Json(_) => "json",
        }
    }
}
