// Credit to https://github.com/hulthe/nu_plugin_msgpack for the original idea, though the
// implementation here is unique, and avoids converting through `rmpv` for somewhat better
// performance.

use nu_engine::command_prelude::*;
use nu_protocol::ast::PathMember;
use rmp::encode::{self as mp, RmpWrite};

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
            WriteError::ShellError(err) => *err,
        })?;

        Ok(Value::binary(out, call.head).into_pipeline_data())
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum WriteError<E: mp::RmpWriteErr> {
    #[error(transparent)]
    ValueWriteError(#[from] mp::ValueWriteError<E>),
    #[error(transparent)]
    ShellError(#[from] Box<ShellError>),
}

impl<E: mp::RmpWriteErr> From<ShellError> for WriteError<E> {
    fn from(value: ShellError) -> Self {
        Box::new(value).into()
    }
}

pub(crate) fn write_value<W>(out: &mut W, value: &Value) -> Result<(), WriteError<W::Error>>
where
    W: RmpWrite,
{
    use mp::ValueWriteError::InvalidMarkerWrite;
    let as_value_write_error = mp::ValueWriteError::from;
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
            // Timestamp extension type
            mp::write_ext_meta(out, 12, -1)?;
            out.write_data_u32(val.timestamp_subsec_nanos())
                .map_err(as_value_write_error)?;
            out.write_data_i64(val.timestamp())
                .map_err(as_value_write_error)?;
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
