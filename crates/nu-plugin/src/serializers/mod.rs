use crate::{
    plugin::PluginEncoder,
    protocol::{PluginInput, PluginOutput},
};
use nu_protocol::ShellError;

pub mod json;
pub mod msgpack;

#[cfg(test)]
mod tests;

#[doc(hidden)]
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

    pub fn to_str(&self) -> &'static str {
        match self {
            Self::Json(_) => "json",
            Self::MsgPack(_) => "msgpack",
        }
    }
}

impl PluginEncoder for EncodingType {
    fn name(&self) -> &str {
        self.to_str()
    }

    fn encode_input(
        &self,
        plugin_input: &PluginInput,
        writer: &mut impl std::io::Write,
    ) -> Result<(), ShellError> {
        match self {
            EncodingType::Json(encoder) => encoder.encode_input(plugin_input, writer),
            EncodingType::MsgPack(encoder) => encoder.encode_input(plugin_input, writer),
        }
    }

    fn decode_input(
        &self,
        reader: &mut impl std::io::BufRead,
    ) -> Result<Option<PluginInput>, ShellError> {
        match self {
            EncodingType::Json(encoder) => encoder.decode_input(reader),
            EncodingType::MsgPack(encoder) => encoder.decode_input(reader),
        }
    }

    fn encode_output(
        &self,
        plugin_output: &PluginOutput,
        writer: &mut impl std::io::Write,
    ) -> Result<(), ShellError> {
        match self {
            EncodingType::Json(encoder) => encoder.encode_output(plugin_output, writer),
            EncodingType::MsgPack(encoder) => encoder.encode_output(plugin_output, writer),
        }
    }

    fn decode_output(
        &self,
        reader: &mut impl std::io::BufRead,
    ) -> Result<Option<PluginOutput>, ShellError> {
        match self {
            EncodingType::Json(encoder) => encoder.decode_output(reader),
            EncodingType::MsgPack(encoder) => encoder.decode_output(reader),
        }
    }
}
