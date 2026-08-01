//! Private wire representation of [`nu_protocol::Value`].
//!
//! Engine `Value` may keep derive-based serde for non-plugin uses. The plugin protocol
//! must only encode values through this module so that:
//!
//! 1. Adding/removing a `Value` variant fails to compile until this mapping is updated.
//! 2. Wire field names/shape are deliberate (e.g. `internal_span` → `span`).
//! 3. Runtime-only fields such as `signals` never appear on the wire.
//!
//! Nested engine payloads inside a value (`ShellError`, `Closure`, `Range`, custom values, …)
//! still use their existing serde derives; prefer protocol snapshots when those change.

use chrono::{DateTime, FixedOffset};
use nu_protocol::{
    CustomValue, Filesize, Range, Record, ShellError, Span, Value, ast::CellPath, engine::Closure,
};
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{MapAccess, Visitor},
    ser::SerializeMap,
};
use std::fmt;

/// Owned wire form of a Nushell [`Value`].
#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum WireValue {
    Bool {
        val: bool,
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Int {
        val: i64,
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Float {
        val: f64,
        #[serde(rename = "span")]
        internal_span: Span,
    },
    String {
        val: String,
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Glob {
        val: String,
        no_expand: bool,
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Filesize {
        val: Filesize,
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Duration {
        val: i64,
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Date {
        val: DateTime<FixedOffset>,
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Range {
        val: Box<Range>,
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Record {
        val: WireRecord,
        #[serde(rename = "span")]
        internal_span: Span,
    },
    List {
        vals: Vec<WireValue>,
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Closure {
        val: Box<Closure>,
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Error {
        error: Box<ShellError>,
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Binary {
        val: Vec<u8>,
        #[serde(rename = "span")]
        internal_span: Span,
    },
    CellPath {
        val: CellPath,
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Custom {
        val: Box<dyn CustomValue>,
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Nothing {
        #[serde(rename = "span")]
        internal_span: Span,
    },
}

/// Map-shaped record on the wire (same encoding as [`Record`]).
#[derive(Debug)]
pub(crate) struct WireRecord {
    entries: Vec<(String, WireValue)>,
}

impl WireRecord {
    fn from_record(record: &Record) -> Self {
        Self {
            entries: record
                .iter()
                .map(|(k, v)| (k.clone(), WireValue::from(v)))
                .collect(),
        }
    }

    fn into_record(self) -> Record {
        self.entries
            .into_iter()
            .map(|(k, v)| (k, Value::from(v)))
            .collect()
    }
}

impl Serialize for WireRecord {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.entries.len()))?;
        for (k, v) in &self.entries {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for WireRecord {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct WireRecordVisitor;

        impl<'de> Visitor<'de> for WireRecordVisitor {
            type Value = WireRecord;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a record map of string keys to wire values")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut entries = Vec::with_capacity(map.size_hint().unwrap_or(0));
                while let Some((key, value)) = map.next_entry::<String, WireValue>()? {
                    if entries.iter().any(|(k, _)| k == &key) {
                        return Err(serde::de::Error::custom(
                            "invalid entry, duplicate keys are not allowed for `Record`",
                        ));
                    }
                    entries.push((key, value));
                }
                Ok(WireRecord { entries })
            }
        }

        deserializer.deserialize_map(WireRecordVisitor)
    }
}

impl From<&Value> for WireValue {
    fn from(value: &Value) -> Self {
        // Exhaustive match: a new Value variant must be handled here (compile fail).
        // `..` is required because Value variants are `non_exhaustive` outside nu-protocol.
        match value {
            Value::Bool {
                val, internal_span, ..
            } => Self::Bool {
                val: *val,
                internal_span: *internal_span,
            },
            Value::Int {
                val, internal_span, ..
            } => Self::Int {
                val: *val,
                internal_span: *internal_span,
            },
            Value::Float {
                val, internal_span, ..
            } => Self::Float {
                val: *val,
                internal_span: *internal_span,
            },
            Value::String {
                val, internal_span, ..
            } => Self::String {
                val: val.clone(),
                internal_span: *internal_span,
            },
            Value::Glob {
                val,
                no_expand,
                internal_span,
                ..
            } => Self::Glob {
                val: val.clone(),
                no_expand: *no_expand,
                internal_span: *internal_span,
            },
            Value::Filesize {
                val, internal_span, ..
            } => Self::Filesize {
                val: *val,
                internal_span: *internal_span,
            },
            Value::Duration {
                val, internal_span, ..
            } => Self::Duration {
                val: *val,
                internal_span: *internal_span,
            },
            Value::Date {
                val, internal_span, ..
            } => Self::Date {
                val: *val,
                internal_span: *internal_span,
            },
            Value::Range {
                val, internal_span, ..
            } => Self::Range {
                val: val.clone(),
                internal_span: *internal_span,
            },
            Value::Record {
                val, internal_span, ..
            } => Self::Record {
                val: WireRecord::from_record(val),
                internal_span: *internal_span,
            },
            Value::List {
                vals,
                internal_span,
                ..
            } => Self::List {
                vals: vals.iter().map(WireValue::from).collect(),
                internal_span: *internal_span,
            },
            Value::Closure {
                val, internal_span, ..
            } => Self::Closure {
                val: val.clone(),
                internal_span: *internal_span,
            },
            Value::Error {
                error,
                internal_span,
                ..
            } => Self::Error {
                error: error.clone(),
                internal_span: *internal_span,
            },
            Value::Binary {
                val, internal_span, ..
            } => Self::Binary {
                val: val.as_ref().to_vec(),
                internal_span: *internal_span,
            },
            Value::CellPath {
                val, internal_span, ..
            } => Self::CellPath {
                val: val.clone(),
                internal_span: *internal_span,
            },
            Value::Custom {
                val, internal_span, ..
            } => match val.clone_value(*internal_span) {
                // Most custom values re-emit themselves; otherwise take the materialized form.
                Value::Custom {
                    val, internal_span, ..
                } => Self::Custom { val, internal_span },
                other => WireValue::from(&other),
            },
            Value::Nothing { internal_span, .. } => Self::Nothing {
                internal_span: *internal_span,
            },
        }
    }
}

impl From<Value> for WireValue {
    fn from(value: Value) -> Self {
        WireValue::from(&value)
    }
}

impl From<WireValue> for Value {
    fn from(value: WireValue) -> Self {
        match value {
            WireValue::Bool { val, internal_span } => Value::bool(val, internal_span),
            WireValue::Int { val, internal_span } => Value::int(val, internal_span),
            WireValue::Float { val, internal_span } => Value::float(val, internal_span),
            WireValue::String { val, internal_span } => Value::string(val, internal_span),
            WireValue::Glob {
                val,
                no_expand,
                internal_span,
            } => Value::glob(val, no_expand, internal_span),
            WireValue::Filesize { val, internal_span } => Value::filesize(val, internal_span),
            WireValue::Duration { val, internal_span } => Value::duration(val, internal_span),
            WireValue::Date { val, internal_span } => Value::date(val, internal_span),
            WireValue::Range { val, internal_span } => Value::range(*val, internal_span),
            WireValue::Record { val, internal_span } => {
                Value::record(val.into_record(), internal_span)
            }
            WireValue::List {
                vals,
                internal_span,
            } => Value::list(vals.into_iter().map(Value::from).collect(), internal_span),
            WireValue::Closure { val, internal_span } => Value::closure(*val, internal_span),
            WireValue::Error {
                error,
                internal_span,
            } => Value::error(*error, internal_span),
            WireValue::Binary { val, internal_span } => Value::binary(val, internal_span),
            WireValue::CellPath { val, internal_span } => Value::cell_path(val, internal_span),
            WireValue::Custom { val, internal_span } => Value::custom(val, internal_span),
            WireValue::Nothing { internal_span } => Value::nothing(internal_span),
        }
    }
}
