//! Nushell type annotations for KDL (YAML-tag analogue).
//!
//! Known annotations: filesize, duration, timestamp (alias: datetime), binary, glob, range, cell-path.
//! JiK also uses structural annotations: array, object.

use base64::{Engine, engine::general_purpose::STANDARD as BASE64_STANDARD};
use chrono::{DateTime, FixedOffset};
use kdl::{KdlEntry, KdlValue};
use nu_engine::command_prelude::*;
use nu_protocol::{Range, ast::CellPath, engine::EngineState};
use num_traits::ToPrimitive;
use std::str::FromStr;

pub(crate) const TY_ARRAY: &str = "array";
pub(crate) const TY_OBJECT: &str = "object";
pub(crate) const TY_FILESIZE: &str = "filesize";
pub(crate) const TY_DURATION: &str = "duration";
pub(crate) const TY_TIMESTAMP: &str = "timestamp";
/// Parse-only alias for [`TY_TIMESTAMP`] (Nu users often say "datetime").
pub(crate) const TY_DATETIME: &str = "datetime";
pub(crate) const TY_BINARY: &str = "binary";
pub(crate) const TY_GLOB: &str = "glob";
pub(crate) const TY_RANGE: &str = "range";
pub(crate) const TY_CELL_PATH: &str = "cell-path";

#[derive(Debug, Clone)]
pub(crate) enum NonRoundtrip {
    Error,
    Null,
    Lossy { engine_state: Box<EngineState> },
}

/// Result of interpreting a KDL value with an optional type annotation.
#[derive(Debug, Clone)]
pub(crate) enum TypedValue {
    /// Plain Nu value (base KDL type or promoted Nu type).
    Plain(Value),
    /// Unknown type annotation preserved for nodes mode: `{ value, type }`.
    Wrapped { value: Value, ty: String },
}

impl TypedValue {
    pub(crate) fn into_value(self, span: Span) -> Value {
        match self {
            Self::Plain(value) => value,
            Self::Wrapped { value, ty } => Value::record(
                record! {
                    "value" => value,
                    "type" => Value::string(ty, span),
                },
                span,
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TypeMode {
    /// Closed dialect: unknown types error. Used for JiK.
    Strict,
    /// Unknown types become wrapped `{value, type}`. Used for nodes.
    PreserveUnknown,
}

/// Convert a KDL entry value + annotation into a Nu value according to policy.
pub(crate) fn apply_type_annotation(
    value: &KdlValue,
    ty: Option<&str>,
    span: Span,
    ignore_types: bool,
    mode: TypeMode,
) -> Result<TypedValue, ShellError> {
    let base = kdl_value_to_base_nu(value, span)?;

    if ignore_types {
        return Ok(TypedValue::Plain(base));
    }

    let Some(ty) = ty else {
        return Ok(TypedValue::Plain(base));
    };

    // Structural JiK annotations are not Nu value types; caller handles them.
    if ty == TY_ARRAY || ty == TY_OBJECT {
        return Ok(TypedValue::Plain(base));
    }

    match promote_known_type(ty, &base, span) {
        Ok(Some(promoted)) => Ok(TypedValue::Plain(promoted)),
        Ok(None) => match mode {
            TypeMode::Strict => Err(ShellError::UnsupportedInput {
                msg: format!(
                    "unknown KDL type annotation '{ty}'; use --ignore-types or --format nodes"
                ),
                input: "value originates from here".into(),
                msg_span: span,
                input_span: span,
            }),
            TypeMode::PreserveUnknown => Ok(TypedValue::Wrapped {
                value: base,
                ty: ty.to_owned(),
            }),
        },
        Err(err) => Err(err),
    }
}

fn promote_known_type(ty: &str, base: &Value, span: Span) -> Result<Option<Value>, ShellError> {
    Ok(Some(match ty {
        TY_FILESIZE => {
            let n = base
                .as_int()
                .map_err(|_| type_payload_error(ty, base, span))?;
            Value::filesize(n, span)
        }
        TY_DURATION => {
            let n = base
                .as_int()
                .map_err(|_| type_payload_error(ty, base, span))?;
            Value::duration(n, span)
        }
        TY_TIMESTAMP | TY_DATETIME => {
            let s = base
                .as_str()
                .map_err(|_| type_payload_error(ty, base, span))?;
            let dt = parse_timestamp(s).map_err(|msg| ShellError::CantConvert {
                to_type: "datetime".into(),
                from_type: "string".into(),
                span: base.span(),
                help: Some(msg),
            })?;
            Value::date(dt, span)
        }
        TY_BINARY => {
            let s = base
                .as_str()
                .map_err(|_| type_payload_error(ty, base, span))?;
            let filtered: String = s.chars().filter(|c| !c.is_whitespace()).collect();
            let bytes =
                BASE64_STANDARD
                    .decode(filtered)
                    .map_err(|err| ShellError::CantConvert {
                        to_type: "binary".into(),
                        from_type: "string".into(),
                        span: base.span(),
                        help: Some(err.to_string()),
                    })?;
            Value::binary(bytes, span)
        }
        TY_GLOB => {
            let s = base
                .as_str()
                .map_err(|_| type_payload_error(ty, base, span))?;
            Value::glob(s, false, span)
        }
        TY_RANGE => {
            let s = base
                .as_str()
                .map_err(|_| type_payload_error(ty, base, span))?;
            let range = Range::from_str(s).map_err(|err| ShellError::CantConvert {
                to_type: "range".into(),
                from_type: "string".into(),
                span: base.span(),
                help: Some(err.to_string()),
            })?;
            Value::range(range, span)
        }
        TY_CELL_PATH => {
            let s = base
                .as_str()
                .map_err(|_| type_payload_error(ty, base, span))?;
            let path = CellPath::from_str(s)
                .map(|cp| cp.with_fallback_span(span))
                .map_err(|err| ShellError::CantConvert {
                    to_type: "cell-path".into(),
                    from_type: "string".into(),
                    span: base.span(),
                    help: Some(err.to_string()),
                })?;
            Value::cell_path(path, span)
        }
        _ => return Ok(None),
    }))
}

fn type_payload_error(ty: &str, base: &Value, _span: Span) -> ShellError {
    ShellError::CantConvert {
        to_type: format!("value for type annotation '{ty}'"),
        from_type: base.get_type().to_string(),
        span: base.span(),
        help: Some(format!("expected a compatible base KDL value for ({ty})")),
    }
}

fn parse_timestamp(s: &str) -> Result<DateTime<FixedOffset>, String> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt);
    }
    if let Ok(dt) = DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f%z") {
        return Ok(dt);
    }
    if let Ok(dt) = DateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f %z") {
        return Ok(dt);
    }
    // Date-only → midnight UTC
    if let Ok(date) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let naive = date
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| format!("invalid date '{s}'"))?;
        return Ok(
            DateTime::<chrono::Utc>::from_naive_utc_and_offset(naive, chrono::Utc).fixed_offset(),
        );
    }
    Err(format!("could not parse timestamp '{s}'"))
}

pub(crate) fn kdl_value_to_base_nu(value: &KdlValue, span: Span) -> Result<Value, ShellError> {
    match value {
        KdlValue::String(val) => Ok(Value::string(val, span)),
        KdlValue::Integer(val) => Ok(Value::int(
            val.to_i64().ok_or(ShellError::UnsupportedInput {
                msg: "integer value is too large to fit in i64".to_owned(),
                input: "value originates from here".to_owned(),
                msg_span: span,
                input_span: span,
            })?,
            span,
        )),
        KdlValue::Float(val) => Ok(Value::float(*val, span)),
        KdlValue::Bool(val) => Ok(Value::bool(*val, span)),
        KdlValue::Null => Ok(Value::nothing(span)),
    }
}

/// Build a KDL entry (argument or property) from a Nu value, applying type annotations.
pub(crate) fn entry_from_nu_value(
    value: &Value,
    prop_name: Option<&str>,
    non_roundtrip: &NonRoundtrip,
    call_span: Span,
) -> Result<KdlEntry, ShellError> {
    // Wrapped unknown type: { value, type }
    if let Value::Record { val, .. } = value
        && val.len() == 2
        && val.get("value").is_some()
        && val.get("type").is_some()
    {
        let inner = val.get("value").expect("checked");
        let ty = val.get("type").expect("checked").as_str().map_err(|_| {
            ShellError::UnsupportedInput {
                msg: "wrapped KDL value 'type' field must be a string".into(),
                input: "value originates from here".into(),
                msg_span: call_span,
                input_span: value.span(),
            }
        })?;
        let (kdl_value, _) = nu_literal_to_kdl(inner, non_roundtrip, call_span)?;
        let mut entry = match prop_name {
            Some(name) => KdlEntry::new_prop(super::identifier_for(name), kdl_value),
            None => KdlEntry::new(kdl_value),
        };
        entry.set_ty(super::identifier_for(ty));
        return Ok(entry);
    }

    let (kdl_value, ty) = nu_literal_to_kdl(value, non_roundtrip, call_span)?;
    let mut entry = match prop_name {
        Some(name) => KdlEntry::new_prop(super::identifier_for(name), kdl_value),
        None => KdlEntry::new(kdl_value),
    };
    if let Some(ty) = ty {
        entry.set_ty(super::identifier_for(ty));
    }
    Ok(entry)
}

/// Convert a Nu scalar (or non-roundtrip case) to KDL value + optional type annotation name.
pub(crate) fn nu_literal_to_kdl(
    value: &Value,
    non_roundtrip: &NonRoundtrip,
    call_span: Span,
) -> Result<(KdlValue, Option<&'static str>), ShellError> {
    match value {
        Value::Bool { val, .. } => Ok((KdlValue::Bool(*val), None)),
        Value::Int { val, .. } => Ok((KdlValue::Integer(*val as i128), None)),
        Value::Float { val, .. } => Ok((KdlValue::Float(*val), None)),
        Value::String { val, .. } => Ok((KdlValue::String(val.clone()), None)),
        Value::Nothing { .. } => Ok((KdlValue::Null, None)),
        Value::Filesize { val, .. } => Ok((KdlValue::Integer(val.get() as i128), Some(TY_FILESIZE))),
        Value::Duration { val, .. } => Ok((KdlValue::Integer(*val as i128), Some(TY_DURATION))),
        Value::Date { val, .. } => Ok((
            KdlValue::String(val.to_rfc3339()),
            Some(TY_TIMESTAMP),
        )),
        Value::Binary { val, .. } => Ok((
            KdlValue::String(BASE64_STANDARD.encode(val)),
            Some(TY_BINARY),
        )),
        Value::Glob { val, .. } => Ok((KdlValue::String(val.clone()), Some(TY_GLOB))),
        Value::Range { val, .. } => Ok((KdlValue::String(val.to_string()), Some(TY_RANGE))),
        Value::CellPath { val, .. } => Ok((KdlValue::String(val.to_string()), Some(TY_CELL_PATH))),
        Value::Closure { val, .. } => match non_roundtrip {
            NonRoundtrip::Error => Err(ShellError::UnsupportedInput {
                msg: "closures cannot round-trip through KDL (use --serialize or --non-roundtrip lossy)".into(),
                input: "value originates from here".into(),
                msg_span: call_span,
                input_span: value.span(),
            }),
            NonRoundtrip::Null => Ok((KdlValue::Null, None)),
            NonRoundtrip::Lossy { engine_state } => Ok((
                KdlValue::String(
                    val.coerce_into_string(engine_state.as_ref(), call_span)?
                        .to_string(),
                ),
                None,
            )),
        },
        Value::Error { error, .. } => match non_roundtrip {
            NonRoundtrip::Error => Err(*error.clone()),
            NonRoundtrip::Null => Ok((KdlValue::Null, None)),
            NonRoundtrip::Lossy { .. } => Ok((KdlValue::String(error.to_string()), None)),
        },
        Value::Custom { val, .. } => match non_roundtrip {
            NonRoundtrip::Error => Err(ShellError::UnsupportedInput {
                msg: format!(
                    "custom value '{}' cannot round-trip through KDL (use --serialize or --non-roundtrip)",
                    val.type_name()
                ),
                input: "value originates from here".into(),
                msg_span: call_span,
                input_span: value.span(),
            }),
            NonRoundtrip::Null => Ok((KdlValue::Null, None)),
            NonRoundtrip::Lossy { .. } => {
                Ok((KdlValue::String(format!("<{}>", val.type_name())), None))
            }
        },
        Value::List { .. } | Value::Record { .. } => Err(ShellError::UnsupportedInput {
            msg: "nested list/record is not a KDL literal".into(),
            input: "value originates from here".into(),
            msg_span: call_span,
            input_span: value.span(),
        }),
    }
}

pub(crate) fn is_kdl_literal(value: &Value) -> bool {
    matches!(
        value,
        Value::Bool { .. }
            | Value::Int { .. }
            | Value::Float { .. }
            | Value::String { .. }
            | Value::Nothing { .. }
            | Value::Filesize { .. }
            | Value::Duration { .. }
            | Value::Date { .. }
            | Value::Binary { .. }
            | Value::Glob { .. }
            | Value::Range { .. }
            | Value::CellPath { .. }
    ) || is_wrapped_typed_value(value)
}

pub(crate) fn is_wrapped_typed_value(value: &Value) -> bool {
    matches!(
        value,
        Value::Record { val, .. }
            if val.len() == 2 && val.get("value").is_some() && val.get("type").is_some()
    )
}
