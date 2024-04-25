use std::io::ErrorKind;

use crate::{
    plugin::{Encoder, PluginEncoder},
    protocol::{PluginInput, PluginOutput},
};
use nu_protocol::ShellError;
use serde::Deserialize;

/// A `PluginEncoder` that enables the plugin to communicate with Nushell with MsgPack
/// serialized data.
///
/// Each message is written as a MessagePack object. There is no message envelope or separator.
#[derive(Clone, Copy, Debug)]
pub struct MsgPackSerializer;

impl PluginEncoder for MsgPackSerializer {
    fn name(&self) -> &str {
        "msgpack"
    }
}

impl Encoder<PluginInput> for MsgPackSerializer {
    fn encode(
        &self,
        plugin_input: &PluginInput,
        writer: &mut impl std::io::Write,
    ) -> Result<(), nu_protocol::ShellError> {
        rmp_serde::encode::write_named(writer, plugin_input).map_err(rmp_encode_err)
    }

    fn decode(
        &self,
        reader: &mut impl std::io::BufRead,
    ) -> Result<Option<PluginInput>, ShellError> {
        let mut de = rmp_serde::Deserializer::new(reader);
        PluginInput::deserialize(&mut de)
            .map(Some)
            .or_else(rmp_decode_err)
    }
}

impl Encoder<PluginOutput> for MsgPackSerializer {
    fn encode(
        &self,
        plugin_output: &PluginOutput,
        writer: &mut impl std::io::Write,
    ) -> Result<(), ShellError> {
        rmp_serde::encode::write_named(writer, plugin_output).map_err(rmp_encode_err)
    }

    fn decode(
        &self,
        reader: &mut impl std::io::BufRead,
    ) -> Result<Option<PluginOutput>, ShellError> {
        let mut de = rmp_serde::Deserializer::new(reader);
        PluginOutput::deserialize(&mut de)
            .map(Some)
            .or_else(rmp_decode_err)
    }
}

/// Handle a msgpack encode error
fn rmp_encode_err(err: rmp_serde::encode::Error) -> ShellError {
    match err {
        rmp_serde::encode::Error::InvalidValueWrite(_) => {
            // I/O error
            ShellError::IOError {
                msg: err.to_string(),
            }
        }
        _ => {
            // Something else
            ShellError::PluginFailedToEncode {
                msg: err.to_string(),
            }
        }
    }
}

/// Handle a msgpack decode error. Returns `Ok(None)` on eof
fn rmp_decode_err<T>(err: rmp_serde::decode::Error) -> Result<Option<T>, ShellError> {
    match err {
        rmp_serde::decode::Error::InvalidMarkerRead(err)
        | rmp_serde::decode::Error::InvalidDataRead(err) => {
            if matches!(err.kind(), ErrorKind::UnexpectedEof) {
                // EOF
                Ok(None)
            } else {
                // I/O error
                Err(ShellError::IOError {
                    msg: err.to_string(),
                })
            }
        }
        _ => {
            // Something else
            Err(ShellError::PluginFailedToDecode {
                msg: err.to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::serializers::tests::generate_tests!(MsgPackSerializer {});
}
