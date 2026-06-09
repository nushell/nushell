//! Explicit serde implementation for [`Value`].
//!
//! Keeping the wire mapping in one place helps avoid accidental protocol drift
//! from derive-generated serialization changes.

use super::{CustomValue, Range, Record, Value};
use crate::{ShellError, Span, ast::CellPath, engine::Closure};
use chrono::{DateTime, FixedOffset};
use nu_utils::SharedCow;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Serialize)]
enum ValueRef<'a> {
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
        val: &'a str,
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Glob {
        val: &'a str,
        no_expand: bool,
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Filesize {
        val: super::Filesize,
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Duration {
        val: i64,
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Date {
        val: &'a DateTime<FixedOffset>,
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Range {
        val: &'a Range,
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Record {
        val: &'a SharedCow<Record>,
        #[serde(rename = "span")]
        internal_span: Span,
    },
    List {
        vals: &'a [Value],
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Closure {
        val: &'a Closure,
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Error {
        error: &'a ShellError,
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Binary {
        val: &'a [u8],
        #[serde(rename = "span")]
        internal_span: Span,
    },
    CellPath {
        val: &'a CellPath,
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Custom {
        val: &'a dyn CustomValue,
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Nothing {
        #[serde(rename = "span")]
        internal_span: Span,
    },
}

#[derive(Deserialize)]
enum ValueDef {
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
        val: super::Filesize,
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
        val: SharedCow<Record>,
        #[serde(rename = "span")]
        internal_span: Span,
    },
    List {
        vals: Vec<Value>,
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

impl<'a> From<&'a Value> for ValueRef<'a> {
    fn from(value: &'a Value) -> Self {
        match value {
            Value::Bool { val, internal_span } => Self::Bool {
                val: *val,
                internal_span: *internal_span,
            },
            Value::Int { val, internal_span } => Self::Int {
                val: *val,
                internal_span: *internal_span,
            },
            Value::Float { val, internal_span } => Self::Float {
                val: *val,
                internal_span: *internal_span,
            },
            Value::String { val, internal_span } => Self::String {
                val,
                internal_span: *internal_span,
            },
            Value::Glob {
                val,
                no_expand,
                internal_span,
            } => Self::Glob {
                val,
                no_expand: *no_expand,
                internal_span: *internal_span,
            },
            Value::Filesize { val, internal_span } => Self::Filesize {
                val: *val,
                internal_span: *internal_span,
            },
            Value::Duration { val, internal_span } => Self::Duration {
                val: *val,
                internal_span: *internal_span,
            },
            Value::Date { val, internal_span } => Self::Date {
                val,
                internal_span: *internal_span,
            },
            Value::Range {
                val, internal_span, ..
            } => Self::Range {
                val,
                internal_span: *internal_span,
            },
            Value::Record { val, internal_span } => Self::Record {
                val,
                internal_span: *internal_span,
            },
            Value::List {
                vals,
                internal_span,
                ..
            } => Self::List {
                vals,
                internal_span: *internal_span,
            },
            Value::Closure { val, internal_span } => Self::Closure {
                val,
                internal_span: *internal_span,
            },
            Value::Error {
                error,
                internal_span,
            } => Self::Error {
                error,
                internal_span: *internal_span,
            },
            Value::Binary { val, internal_span } => Self::Binary {
                val,
                internal_span: *internal_span,
            },
            Value::CellPath { val, internal_span } => Self::CellPath {
                val,
                internal_span: *internal_span,
            },
            Value::Custom { val, internal_span } => Self::Custom {
                val: val.as_ref(),
                internal_span: *internal_span,
            },
            Value::Nothing { internal_span } => Self::Nothing {
                internal_span: *internal_span,
            },
        }
    }
}

impl From<ValueDef> for Value {
    fn from(value: ValueDef) -> Self {
        match value {
            ValueDef::Bool { val, internal_span } => Self::Bool { val, internal_span },
            ValueDef::Int { val, internal_span } => Self::Int { val, internal_span },
            ValueDef::Float { val, internal_span } => Self::Float { val, internal_span },
            ValueDef::String { val, internal_span } => Self::String { val, internal_span },
            ValueDef::Glob {
                val,
                no_expand,
                internal_span,
            } => Self::Glob {
                val,
                no_expand,
                internal_span,
            },
            ValueDef::Filesize { val, internal_span } => Self::Filesize { val, internal_span },
            ValueDef::Duration { val, internal_span } => Self::Duration { val, internal_span },
            ValueDef::Date { val, internal_span } => Self::Date { val, internal_span },
            ValueDef::Range { val, internal_span } => Self::Range {
                val,
                signals: None,
                internal_span,
            },
            ValueDef::Record { val, internal_span } => Self::Record { val, internal_span },
            ValueDef::List {
                vals,
                internal_span,
            } => Self::List {
                vals,
                signals: None,
                internal_span,
            },
            ValueDef::Closure { val, internal_span } => Self::Closure { val, internal_span },
            ValueDef::Error {
                error,
                internal_span,
            } => Self::Error {
                error,
                internal_span,
            },
            ValueDef::Binary { val, internal_span } => Self::Binary { val, internal_span },
            ValueDef::CellPath { val, internal_span } => Self::CellPath { val, internal_span },
            ValueDef::Custom { val, internal_span } => Self::Custom { val, internal_span },
            ValueDef::Nothing { internal_span } => Self::Nothing { internal_span },
        }
    }
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ValueRef::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(ValueDef::deserialize(deserializer)?.into())
    }
}
