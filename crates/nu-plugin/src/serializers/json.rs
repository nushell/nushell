use crate::{
    plugin::{Encoder, PluginEncoder},
    protocol::{PluginInput, PluginOutput},
};
use nu_protocol::ShellError;
use serde::Deserialize;

/// A `PluginEncoder` that enables the plugin to communicate with Nushell with JSON
/// serialized data.
///
/// Each message in the stream is followed by a newline when serializing, but is not required for
/// deserialization. The output is not pretty printed and each object does not contain newlines.
/// If it is more convenient, a plugin may choose to separate messages by newline.
#[derive(Clone, Copy, Debug)]
pub struct JsonSerializer;

impl PluginEncoder for JsonSerializer {
    fn name(&self) -> &str {
        "json"
    }
}

impl Encoder<PluginInput> for JsonSerializer {
    fn encode(
        &self,
        plugin_input: &PluginInput,
        writer: &mut impl std::io::Write,
    ) -> Result<(), nu_protocol::ShellError> {
        serde_json::to_writer(&mut *writer, plugin_input).map_err(json_encode_err)?;
        writer.write_all(b"\n").map_err(|err| ShellError::IOError {
            msg: err.to_string(),
        })?;

        if let Some(mut f) = std::env::var("NU_PLUGIN_INPUT_JSON").ok().and_then(|path| {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .ok()
        }) {
            // If this environment variable is set to a writable file, append the JSON format plugin input.
            use std::io::Write;

            _ = serde_json::to_writer(&mut f, plugin_input).map_err(json_encode_err);
            _ = f.write_all(b"\n\n");
        };

        Ok(())
    }

    fn decode(
        &self,
        reader: &mut impl std::io::BufRead,
    ) -> Result<Option<PluginInput>, nu_protocol::ShellError> {
        let mut de = serde_json::Deserializer::from_reader(reader);
        PluginInput::deserialize(&mut de)
            .map(Some)
            .or_else(json_decode_err)
    }
}

impl Encoder<PluginOutput> for JsonSerializer {
    fn encode(
        &self,
        plugin_output: &PluginOutput,
        writer: &mut impl std::io::Write,
    ) -> Result<(), ShellError> {
        serde_json::to_writer(&mut *writer, plugin_output).map_err(json_encode_err)?;
        writer.write_all(b"\n").map_err(|err| ShellError::IOError {
            msg: err.to_string(),
        })
    }

    fn decode(
        &self,
        reader: &mut impl std::io::BufRead,
    ) -> Result<Option<PluginOutput>, ShellError> {
        let mut de = serde_json::Deserializer::from_reader(reader);
        let plugin_output = PluginOutput::deserialize(&mut de)
            .map(Some)
            .or_else(json_decode_err)?;

        if let Some((mut f, path)) = std::env::var("NU_PLUGIN_OUTPUT_JSON")
            .ok()
            .and_then(|path| {
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
                    .ok()
                    .map(|f| (f, path))
            })
        {
            // If this environment variable is set to a writable file, append the JSON format plugin output.
            use std::io::Write;

            match plugin_output {
                Some(ref plugin_output) => {
                    serde_json::to_writer(&mut f, plugin_output)
                        .unwrap_or_else(|e| panic!("dump plugin output JSON to {}: {}", &path, e));
                    f.write_all(b"\n\n").unwrap_or_else(|e| {
                        panic!("write terminating newline to {}: {}", &path, e)
                    });
                }
                None => {
                    f.write_all(b"\n\n").unwrap_or_else(|e| {
                        panic!("write terminating newline to {}: {}", &path, e)
                    });
                }
            }
        }

        Ok(plugin_output)
    }
}

/// Handle a `serde_json` encode error.
fn json_encode_err(err: serde_json::Error) -> ShellError {
    if err.is_io() {
        ShellError::IOError {
            msg: err.to_string(),
        }
    } else {
        ShellError::PluginFailedToEncode {
            msg: err.to_string(),
        }
    }
}

/// Handle a `serde_json` decode error. Returns `Ok(None)` on eof.
fn json_decode_err<T>(err: serde_json::Error) -> Result<Option<T>, ShellError> {
    if err.is_eof() {
        Ok(None)
    } else if err.is_io() {
        Err(ShellError::IOError {
            msg: err.to_string(),
        })
    } else {
        Err(ShellError::PluginFailedToDecode {
            msg: err.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::serializers::tests::generate_tests!(JsonSerializer {});

    #[test]
    fn json_ends_in_newline() {
        let mut out = vec![];
        JsonSerializer {}
            .encode(&PluginInput::Call(0, PluginCall::Signature), &mut out)
            .expect("serialization error");
        let string = std::str::from_utf8(&out).expect("utf-8 error");
        assert!(
            string.ends_with('\n'),
            "doesn't end with newline: {:?}",
            string
        );
    }

    #[test]
    fn json_has_no_other_newlines() {
        let mut out = vec![];
        // use something deeply nested, to try to trigger any pretty printing
        let output = PluginOutput::Stream(StreamMessage::Data(
            0,
            StreamData::List(Value::test_list(vec![
                Value::test_int(4),
                // in case escaping failed
                Value::test_string("newline\ncontaining\nstring"),
            ])),
        ));
        JsonSerializer {}
            .encode(&output, &mut out)
            .expect("serialization error");
        let string = std::str::from_utf8(&out).expect("utf-8 error");
        assert_eq!(1, string.chars().filter(|ch| *ch == '\n').count());
    }
}
