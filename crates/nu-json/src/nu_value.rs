use nu_protocol::{
    CustomValue, FromValue, IntoValue, Record, ShellError, Span, Type,
    engine::{Closure, EngineState},
};

use crate::Value as JsonValue;
use nu_protocol::Value as NuValue;

impl IntoValue for JsonValue {
    fn into_value(self, span: Span) -> NuValue {
        match self {
            JsonValue::String(s) => NuValue::string(s, span),
            JsonValue::Bool(b) => NuValue::bool(b, span),
            JsonValue::F64(f) => NuValue::float(f, span),
            JsonValue::I64(i) => NuValue::int(i, span),
            JsonValue::Null => NuValue::nothing(span),
            JsonValue::U64(u) => match i64::try_from(u) {
                Ok(i) => NuValue::int(i, span),
                Err(_) => NuValue::float(u as f64, span),
            },
            JsonValue::Array(array) => NuValue::list(
                array.into_iter().map(|x| x.into_value(span)).collect(),
                span,
            ),
            JsonValue::Object(k) => NuValue::record(
                k.into_iter()
                    .map(|(k, v)| (k, v.into_value(span)))
                    .collect(),
                span,
            ),
        }
    }
}

#[derive(Clone, Copy)]
enum SerializeMode<'e> {
    Simple,
    Coercing { engine_state: &'e EngineState },
}

impl FromValue for JsonValue {
    fn from_value(v: NuValue) -> Result<Self, ShellError> {
        Self::from_value_impl(v, SerializeMode::Simple)
    }
}

impl JsonValue {
    pub fn from_value_serialized(
        v: NuValue,
        engine_state: &EngineState,
    ) -> Result<Self, ShellError> {
        Self::from_value_impl(v, SerializeMode::Coercing { engine_state })
    }

    fn from_value_impl(v: NuValue, mode: SerializeMode<'_>) -> Result<Self, ShellError> {
        let span = v.span();
        Ok(match v {
            NuValue::Bool { val, .. } => JsonValue::Bool(val),
            NuValue::Int { val, .. } => JsonValue::I64(val),
            NuValue::Float { val, .. } => JsonValue::F64(val),
            NuValue::String { val, .. } => JsonValue::String(val),
            NuValue::Glob { val, .. } => JsonValue::String(val.to_string()),
            NuValue::Filesize { val, .. } => JsonValue::I64(val.get()),
            NuValue::Duration { val, .. } => JsonValue::I64(val),
            NuValue::Date { val, .. } => JsonValue::String(val.to_rfc3339()),
            NuValue::Range { val, .. } => JsonValue::String(val.to_string()),
            NuValue::Record { val, .. } => record_into_json_value(val.into_owned(), mode)?,
            NuValue::List { vals, .. } => list_into_json_value(vals, mode)?,
            NuValue::Closure { val, .. } => closure_into_json_value(*val, mode, span)?,
            NuValue::Error { error, .. } => return Err(*error),
            NuValue::Binary { val, .. } => binary_into_json_value(&val),
            NuValue::CellPath { val, .. } => JsonValue::String(val.to_string()),
            NuValue::Custom { val, .. } => custom_into_json_value(val, mode, span)?,
            NuValue::Nothing { .. } => JsonValue::Null,
        })
    }
}

fn record_into_json_value(
    record: Record,
    mode: SerializeMode<'_>,
) -> Result<JsonValue, ShellError> {
    Ok(JsonValue::Object(
        record
            .into_iter()
            .map(|(k, v)| JsonValue::from_value_impl(v, mode).map(|v| (k, v)))
            .collect::<Result<_, _>>()?,
    ))
}

fn list_into_json_value(
    vals: impl IntoIterator<Item = NuValue>,
    mode: SerializeMode<'_>,
) -> Result<JsonValue, ShellError> {
    Ok(JsonValue::Array(
        vals.into_iter()
            .map(|v| JsonValue::from_value_impl(v, mode))
            .collect::<Result<_, _>>()?,
    ))
}

fn closure_into_json_value(
    val: Closure,
    mode: SerializeMode<'_>,
    span: Span,
) -> Result<JsonValue, ShellError> {
    match mode {
        SerializeMode::Simple => Err(ShellError::CantConvert {
            to_type: Type::String.to_string(),
            from_type: Type::Closure.to_string(),
            span,
            help: None,
        }),
        SerializeMode::Coercing { engine_state } => Ok(JsonValue::String(
            val.coerce_into_string(engine_state, span)?.to_string(),
        )),
    }
}

fn binary_into_json_value(val: &[u8]) -> JsonValue {
    JsonValue::Array(
        val.iter()
            .copied()
            .map(|b| JsonValue::U64(b as u64))
            .collect(),
    )
}

fn custom_into_json_value(
    val: Box<dyn CustomValue>,
    mode: SerializeMode,
    span: Span,
) -> Result<JsonValue, ShellError> {
    JsonValue::from_value_impl(val.to_base_value(span)?, mode)
}
