use nu_plugin_protocol::{PluginInput, PluginOutput};
use nu_protocol::ShellError;

pub mod json;
pub mod msgpack;

#[cfg(test)]
mod tests;

/// Encoder for a specific message type. Usually implemented on [`PluginInput`]
/// and [`PluginOutput`].
pub trait Encoder<T>: Clone + Send + Sync {
    /// Serialize a value in the [`PluginEncoder`]s format
    ///
    /// Returns [`ShellError::Io`] if there was a problem writing, or
    /// [`ShellError::PluginFailedToEncode`] for a serialization error.
    fn encode(&self, data: &T, writer: &mut impl std::io::Write) -> Result<(), ShellError>;

    /// Deserialize a value from the [`PluginEncoder`]'s format
    ///
    /// Returns `None` if there is no more output to receive.
    ///
    /// Returns [`ShellError::Io`] if there was a problem reading, or
    /// [`ShellError::PluginFailedToDecode`] for a deserialization error.
    fn decode(&self, reader: &mut impl std::io::BufRead) -> Result<Option<T>, ShellError>;
}

/// Encoding scheme that defines a plugin's communication protocol with Nu
pub trait PluginEncoder: Encoder<PluginInput> + Encoder<PluginOutput> {
    /// The name of the encoder (e.g., `json`)
    fn name(&self) -> &str;
}

/// Enum that supports all of the plugin serialization formats.
#[derive(Clone, Copy, Debug)]
pub enum EncodingType {
    Json(json::JsonSerializer),
    MsgPack(msgpack::MsgPackSerializer),
}

impl EncodingType {
    /// Determine the plugin encoding type from the provided byte string (either `b"json"` or
    /// `b"msgpack"`).
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
