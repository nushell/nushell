// Credit to https://github.com/hulthe/nu_plugin_msgpack for the original idea, though the
// implementation here is unique.

use std::io;

use byteorder::{BigEndian, WriteBytesExt};
use nu_engine::command_prelude::*;
use nu_protocol::{Signals, Spanned, ast::PathMember, shell_error::io::IoError};
use rmp::encode as mp;

/// Max recursion depth
const MAX_DEPTH: usize = 50;

#[derive(Clone)]
pub struct ToMsgpack;

impl Command for ToMsgpack {
    fn name(&self) -> &str {
        "to msgpack"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(Type::Any, Type::Binary)
            .switch(
                "serialize",
                "serialize nushell types that cannot be deserialized",
                Some('s'),
            )
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Convert Nu values into MessagePack."
    }

    fn extra_description(&self) -> &str {
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
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let metadata = input
            .metadata()
            .unwrap_or_default()
            .with_content_type(Some("application/x-msgpack".into()));

        let value_span = input.span().unwrap_or(call.head);
        let value = input.into_value(value_span)?;
        let mut out = vec![];

        let serialize_types = call.has_flag(engine_state, stack, "serialize")?;

        write_value(
            &mut out,
            &value,
            0,
            engine_state,
            call.head,
            serialize_types,
        )?;

        Ok(Value::binary(out, call.head).into_pipeline_data_with_metadata(Some(metadata)))
    }
}

#[derive(Debug)]
pub(crate) enum WriteError {
    MaxDepth(Span),
    Rmp(mp::ValueWriteError<io::Error>, Span),
    Io(io::Error, Span),
    Shell(Box<ShellError>),
}

impl From<Spanned<mp::ValueWriteError<io::Error>>> for WriteError {
    fn from(v: Spanned<mp::ValueWriteError<io::Error>>) -> Self {
        Self::Rmp(v.item, v.span)
    }
}

impl From<Spanned<io::Error>> for WriteError {
    fn from(v: Spanned<io::Error>) -> Self {
        Self::Io(v.item, v.span)
    }
}

impl From<Box<ShellError>> for WriteError {
    fn from(v: Box<ShellError>) -> Self {
        Self::Shell(v)
    }
}

impl From<ShellError> for WriteError {
    fn from(value: ShellError) -> Self {
        Box::new(value).into()
    }
}

impl From<WriteError> for ShellError {
    fn from(value: WriteError) -> Self {
        match value {
            WriteError::MaxDepth(span) => ShellError::GenericError {
                error: "MessagePack data is nested too deeply".into(),
                msg: format!("exceeded depth limit ({MAX_DEPTH})"),
                span: Some(span),
                help: None,
                inner: vec![],
            },
            WriteError::Rmp(err, span) => ShellError::GenericError {
                error: "Failed to encode MessagePack data".into(),
                msg: err.to_string(),
                span: Some(span),
                help: None,
                inner: vec![],
            },
            WriteError::Io(err, span) => ShellError::Io(IoError::new(err, span, None)),
            WriteError::Shell(err) => *err,
        }
    }
}

pub(crate) fn write_value(
    out: &mut impl io::Write,
    value: &Value,
    depth: usize,
    engine_state: &EngineState,
    call_span: Span,
    serialize_types: bool,
) -> Result<(), WriteError> {
    use mp::ValueWriteError::InvalidMarkerWrite;
    let span = value.span();
    // Prevent stack overflow
    if depth >= MAX_DEPTH {
        return Err(WriteError::MaxDepth(span));
    }
    match value {
        Value::Bool { val, .. } => {
            mp::write_bool(out, *val)
                .map_err(InvalidMarkerWrite)
                .err_span(span)?;
        }
        Value::Int { val, .. } => {
            mp::write_sint(out, *val).err_span(span)?;
        }
        Value::Float { val, .. } => {
            mp::write_f64(out, *val).err_span(span)?;
        }
        Value::Filesize { val, .. } => {
            mp::write_sint(out, val.get()).err_span(span)?;
        }
        Value::Duration { val, .. } => {
            mp::write_sint(out, *val).err_span(span)?;
        }
        Value::Date { val, .. } => {
            if val.timestamp_subsec_nanos() == 0
                && val.timestamp() >= 0
                && val.timestamp() < u32::MAX as i64
            {
                // Timestamp extension type, 32-bit. u32 seconds since UNIX epoch only.
                mp::write_ext_meta(out, 4, -1).err_span(span)?;
                out.write_u32::<BigEndian>(val.timestamp() as u32)
                    .err_span(span)?;
            } else {
                // Timestamp extension type, 96-bit. u32 nanoseconds and i64 seconds.
                mp::write_ext_meta(out, 12, -1).err_span(span)?;
                out.write_u32::<BigEndian>(val.timestamp_subsec_nanos())
                    .err_span(span)?;
                out.write_i64::<BigEndian>(val.timestamp()).err_span(span)?;
            }
        }
        Value::Range { val, .. } => {
            // Convert range to list
            write_value(
                out,
                &Value::list(val.into_range_iter(span, Signals::empty()).collect(), span),
                depth,
                engine_state,
                call_span,
                serialize_types,
            )?;
        }
        Value::String { val, .. } => {
            mp::write_str(out, val).err_span(span)?;
        }
        Value::Glob { val, .. } => {
            mp::write_str(out, val).err_span(span)?;
        }
        Value::Record { val, .. } => {
            mp::write_map_len(out, convert(val.len(), span)?).err_span(span)?;
            for (k, v) in val.iter() {
                mp::write_str(out, k).err_span(span)?;
                write_value(out, v, depth + 1, engine_state, call_span, serialize_types)?;
            }
        }
        Value::List { vals, .. } => {
            mp::write_array_len(out, convert(vals.len(), span)?).err_span(span)?;
            for val in vals {
                write_value(
                    out,
                    val,
                    depth + 1,
                    engine_state,
                    call_span,
                    serialize_types,
                )?;
            }
        }
        Value::Nothing { .. } => {
            mp::write_nil(out)
                .map_err(InvalidMarkerWrite)
                .err_span(span)?;
        }
        Value::Closure { val, .. } => {
            if serialize_types {
                let closure_string = val
                    .coerce_into_string(engine_state, span)
                    .map_err(|err| WriteError::Shell(Box::new(err)))?;
                mp::write_str(out, &closure_string).err_span(span)?;
            } else {
                return Err(WriteError::Shell(Box::new(ShellError::UnsupportedInput {
                    msg: "closures are currently not deserializable (use --serialize to serialize as a string)".into(),
                    input: "value originates from here".into(),
                    msg_span: call_span,
                    input_span: span,
                })));
            }
        }
        Value::Error { error, .. } => {
            return Err(WriteError::Shell(error.clone()));
        }
        Value::CellPath { val, .. } => {
            // Write as a list of strings/ints
            mp::write_array_len(out, convert(val.members.len(), span)?).err_span(span)?;
            for member in &val.members {
                match member {
                    PathMember::String { val, .. } => {
                        mp::write_str(out, val).err_span(span)?;
                    }
                    PathMember::Int { val, .. } => {
                        mp::write_uint(out, *val as u64).err_span(span)?;
                    }
                }
            }
        }
        Value::Binary { val, .. } => {
            mp::write_bin(out, val).err_span(span)?;
        }
        Value::Custom { val, .. } => {
            write_value(
                out,
                &val.to_base_value(span)?,
                depth,
                engine_state,
                call_span,
                serialize_types,
            )?;
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
    use nu_cmd_lang::eval_pipeline_without_terminal_expression;

    use crate::{Get, Metadata};

    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(ToMsgpack {})
    }

    #[test]
    fn test_content_type_metadata() {
        let mut engine_state = Box::new(EngineState::new());
        let delta = {
            // Base functions that are needed for testing
            // Try to keep this working set small to keep tests running as fast as possible
            let mut working_set = StateWorkingSet::new(&engine_state);

            working_set.add_decl(Box::new(ToMsgpack {}));
            working_set.add_decl(Box::new(Metadata {}));
            working_set.add_decl(Box::new(Get {}));

            working_set.render()
        };

        engine_state
            .merge_delta(delta)
            .expect("Error merging delta");

        let cmd = "{a: 1 b: 2} | to msgpack | metadata | get content_type | $in";
        let result = eval_pipeline_without_terminal_expression(
            cmd,
            std::env::temp_dir().as_ref(),
            &mut engine_state,
        );
        assert_eq!(
            Value::test_string("application/x-msgpack"),
            result.expect("There should be a result")
        );
    }
}
