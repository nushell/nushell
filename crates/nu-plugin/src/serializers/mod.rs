use crate::plugin::Encoder;
use nu_protocol::ShellError;

pub mod json;
pub mod msgpack;

#[cfg(test)]
mod tests;

#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
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
}

impl<T> Encoder<T> for EncodingType
where
    json::JsonSerializer: Encoder<T>,
    msgpack::MsgPackSerializer: Encoder<T>,
{
    fn encode(&self, data: &T, writer: &mut impl std::io::Write) -> Result<(), ShellError> {
        match self {
            EncodingType::Json(encoder) => encoder.encode(data, writer),
            EncodingType::MsgPack(encoder) => encoder.encode(data, writer),
        }
    }

    fn decode(&self, reader: &mut impl std::io::BufRead) -> Result<Option<T>, ShellError> {
        match self {
            EncodingType::Json(encoder) => encoder.decode(reader),
            EncodingType::MsgPack(encoder) => encoder.decode(reader),
        }
    }
}
