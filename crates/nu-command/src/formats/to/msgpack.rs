// Credit to https://github.com/hulthe/nu_plugin_msgpack for the original idea, though the
// implementation here is unique.

use std::io;

use byteorder::{BigEndian, WriteBytesExt};
use nu_engine::command_prelude::*;
use nu_protocol::ast::PathMember;
use rmp::encode as mp;

#[derive(Clone)]
pub struct ToMsgpack;

impl Command for ToMsgpack {
    fn name(&self) -> &str {
        "to msgpack"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(Type::Any, Type::Binary)
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "Convert Nu values into MessagePack."
    }

    fn extra_usage(&self) -> &str {
        r#"
Not all values are representable as MessagePack.

The datetime extension type is used for dates. Binaries are represented with
the native MessagePack binary type. Most other types are represented in an
analogous way to `to json`, and may not convert to the exact same type when
deserialized with `from msgpack`.

MessagePack: https://msgpack.org/
"#
        .trim()
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert a list of values to MessagePack",
                example: "[foo, 42, false] | to msgpack",
                result: Some(Value::test_binary(b"\x93\xA3\x66\x6F\x6F\x2A\xC2")),
            },
            Example {
                description: "Convert a range to a MessagePack array",
                example: "1..10 | to msgpack",
                result: Some(Value::test_binary(b"\x9A\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A"))
            },
            Example {
                description: "Convert a table to MessagePack",
                example: "[
        [event_name time];
        ['Apollo 11 Landing' 1969-07-24T16:50:35]
        ['Nushell first commit' 2019-05-10T09:59:12-07:00]
    ] | to msgpack",
                result: Some(Value::test_binary(b"\x92\x82\xAA\x65\x76\x65\x6E\x74\x5F\x6E\x61\x6D\x65\xB1\x41\x70\x6F\x6C\x6C\x6F\x20\x31\x31\x20\x4C\x61\x6E\x64\x69\x6E\x67\xA4\x74\x69\x6D\x65\xC7\x0C\xFF\x00\x00\x00\x00\xFF\xFF\xFF\xFF\xFF\x2C\xAB\x5B\x82\xAA\x65\x76\x65\x6E\x74\x5F\x6E\x61\x6D\x65\xB4\x4E\x75\x73\x68\x65\x6C\x6C\x20\x66\x69\x72\x73\x74\x20\x63\x6F\x6D\x6D\x69\x74\xA4\x74\x69\x6D\x65\xD6\xFF\x5C\xD5\xAD\xE0")),
            },
        ]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let value_span = input.span().unwrap_or(call.head);
        let value = input.into_value(value_span);
        let mut out = vec![];

        write_value(&mut out, &value).map_err(|err| match err {
            WriteError::ValueWriteError(err) => ShellError::GenericError {
                error: "Failed to encode MessagePack data".into(),
                msg: err.to_string(),
                span: Some(value_span),
                help: None,
                inner: vec![],
            },
            WriteError::IoError(err) => err.into_spanned(value_span).into(),
            WriteError::ShellError(err) => *err,
        })?;

        Ok(Value::binary(out, call.head).into_pipeline_data())
    }
}

#[derive(Debug)]
pub(crate) enum WriteError {
    ValueWriteError(mp::ValueWriteError<io::Error>),
    IoError(io::Error),
    ShellError(Box<ShellError>),
}

impl From<mp::ValueWriteError<io::Error>> for WriteError {
    fn from(v: mp::ValueWriteError<io::Error>) -> Self {
        Self::ValueWriteError(v)
    }
}

impl From<io::Error> for WriteError {
    fn from(v: io::Error) -> Self {
        Self::IoError(v)
    }
}

impl From<Box<ShellError>> for WriteError {
    fn from(v: Box<ShellError>) -> Self {
        Self::ShellError(v)
    }
}

impl From<ShellError> for WriteError {
    fn from(value: ShellError) -> Self {
        Box::new(value).into()
    }
}

pub(crate) fn write_value(out: &mut impl io::Write, value: &Value) -> Result<(), WriteError> {
    use mp::ValueWriteError::InvalidMarkerWrite;
    let span = value.span();
    match value {
        Value::Bool { val, .. } => {
            mp::write_bool(out, *val).map_err(InvalidMarkerWrite)?;
        }
        Value::Int { val, .. } => {
            mp::write_sint(out, *val)?;
        }
        Value::Float { val, .. } => {
            mp::write_f64(out, *val)?;
        }
        Value::Filesize { val, .. } => {
            mp::write_sint(out, *val)?;
        }
        Value::Duration { val, .. } => {
            mp::write_sint(out, *val)?;
        }
        Value::Date { val, .. } => {
            if val.timestamp_subsec_nanos() == 0
                && val.timestamp() >= 0
                && val.timestamp() < u32::MAX as i64
            {
                // Timestamp extension type, 32-bit. u32 seconds since UNIX epoch only.
                mp::write_ext_meta(out, 4, -1)?;
                out.write_u32::<BigEndian>(val.timestamp() as u32)?;
            } else {
                // Timestamp extension type, 96-bit. u32 nanoseconds and i64 seconds.
                mp::write_ext_meta(out, 12, -1)?;
                out.write_u32::<BigEndian>(val.timestamp_subsec_nanos())?;
                out.write_i64::<BigEndian>(val.timestamp())?;
            }
        }
        Value::Range { val, .. } => {
            // Convert range to list
            write_value(
                out,
                &Value::list(val.into_range_iter(span, None).collect(), span),
            )?;
        }
        Value::String { val, .. } => {
            mp::write_str(out, val)?;
        }
        Value::Glob { val, .. } => {
            mp::write_str(out, val)?;
        }
        Value::Record { val, .. } => {
            mp::write_map_len(out, convert(val.len(), span)?)?;
            for (k, v) in val.iter() {
                mp::write_str(out, k)?;
                write_value(out, v)?;
            }
        }
        Value::List { vals, .. } => {
            mp::write_array_len(out, convert(vals.len(), span)?)?;
            for val in vals {
                write_value(out, val)?;
            }
        }
        Value::Nothing { .. } => {
            mp::write_nil(out).map_err(InvalidMarkerWrite)?;
        }
        Value::Closure { .. } => {
            // Closures can't be converted
            mp::write_nil(out).map_err(InvalidMarkerWrite)?;
        }
        Value::Error { error, .. } => {
            return Err(WriteError::ShellError(error.clone()));
        }
        Value::CellPath { val, .. } => {
            // Write as a list of strings/ints
            mp::write_array_len(out, convert(val.members.len(), span)?)?;
            for member in &val.members {
                match member {
                    PathMember::String { val, .. } => {
                        mp::write_str(out, val)?;
                    }
                    PathMember::Int { val, .. } => {
                        mp::write_uint(out, *val as u64)?;
                    }
                }
            }
        }
        Value::Binary { val, .. } => {
            mp::write_bin(out, val)?;
        }
        Value::Custom { val, .. } => {
            write_value(out, &val.to_base_value(span)?)?;
        }
        Value::LazyRecord { val, .. } => {
            write_value(out, &val.collect()?)?;
        }
    }
    Ok(())
}

fn convert<T, U>(value: T, span: Span) -> Result<U, ShellError>
where
    U: TryFrom<T>,
    <U as TryFrom<T>>::Error: std::fmt::Display,
{
    value
        .try_into()
        .map_err(|err: <U as TryFrom<T>>::Error| ShellError::GenericError {
            error: "Value not compatible with MessagePack".into(),
            msg: err.to_string(),
            span: Some(span),
            help: None,
            inner: vec![],
        })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(ToMsgpack {})
    }
}
