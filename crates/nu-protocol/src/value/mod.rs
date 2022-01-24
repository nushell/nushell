mod custom_value;
mod from;
mod from_value;
mod range;
mod stream;
mod unit;

use byte_unit::ByteUnit;
use chrono::{DateTime, FixedOffset};
use chrono_humanize::HumanTime;
pub use from_value::FromValue;
use indexmap::map::IndexMap;
use num_format::{Locale, ToFormattedString};
pub use range::*;
use serde::{Deserialize, Serialize};
pub use stream::*;
use sys_locale::get_locale;
pub use unit::*;

use std::collections::HashMap;
use std::path::PathBuf;
use std::{cmp::Ordering, fmt::Debug};

use crate::ast::{CellPath, PathMember};
use crate::{did_you_mean, span, BlockId, Config, Span, Spanned, Type, VarId};

use crate::ast::Operator;
pub use custom_value::CustomValue;

use crate::ShellError;

/// Core structured values that pass through the pipeline in engine-q
#[derive(Debug, Serialize, Deserialize)]
pub enum Value {
    Bool {
        val: bool,
        span: Span,
    },
    Int {
        val: i64,
        span: Span,
    },
    Filesize {
        val: i64,
        span: Span,
    },
    Duration {
        val: i64,
        span: Span,
    },
    Date {
        val: DateTime<FixedOffset>,
        span: Span,
    },
    Range {
        val: Box<Range>,
        span: Span,
    },
    Float {
        val: f64,
        span: Span,
    },
    String {
        val: String,
        span: Span,
    },
    Record {
        cols: Vec<String>,
        vals: Vec<Value>,
        span: Span,
    },
    List {
        vals: Vec<Value>,
        span: Span,
    },
    Block {
        val: BlockId,
        captures: HashMap<VarId, Value>,
        span: Span,
    },
    Nothing {
        span: Span,
    },
    Error {
        error: ShellError,
    },
    Binary {
        val: Vec<u8>,
        span: Span,
    },
    CellPath {
        val: CellPath,
        span: Span,
    },
    CustomValue {
        val: Box<dyn CustomValue>,
        span: Span,
    },
}

impl Clone for Value {
    fn clone(&self) -> Self {
        match self {
            Value::Bool { val, span } => Value::Bool {
                val: *val,
                span: *span,
            },
            Value::Int { val, span } => Value::Int {
                val: *val,
                span: *span,
            },
            Value::Filesize { val, span } => Value::Filesize {
                val: *val,
                span: *span,
            },
            Value::Duration { val, span } => Value::Duration {
                val: *val,
                span: *span,
            },
            Value::Date { val, span } => Value::Date {
                val: *val,
                span: *span,
            },
            Value::Range { val, span } => Value::Range {
                val: val.clone(),
                span: *span,
            },
            Value::Float { val, span } => Value::Float {
                val: *val,
                span: *span,
            },
            Value::String { val, span } => Value::String {
                val: val.clone(),
                span: *span,
            },
            Value::Record { cols, vals, span } => Value::Record {
                cols: cols.clone(),
                vals: vals.clone(),
                span: *span,
            },
            Value::List { vals, span } => Value::List {
                vals: vals.clone(),
                span: *span,
            },
            Value::Block {
                val,
                captures,
                span,
            } => Value::Block {
                val: *val,
                captures: captures.clone(),
                span: *span,
            },
            Value::Nothing { span } => Value::Nothing { span: *span },
            Value::Error { error } => Value::Error {
                error: error.clone(),
            },
            Value::Binary { val, span } => Value::Binary {
                val: val.clone(),
                span: *span,
            },
            Value::CellPath { val, span } => Value::CellPath {
                val: val.clone(),
                span: *span,
            },
            Value::CustomValue { val, span } => val.clone_value(*span),
        }
    }
}

impl Value {
    pub fn as_string(&self) -> Result<String, ShellError> {
        match self {
            Value::String { val, .. } => Ok(val.to_string()),
            Value::Binary { val, .. } => Ok(match std::str::from_utf8(val) {
                Ok(s) => s.to_string(),
                Err(_) => {
                    return Err(ShellError::CantConvert(
                        "binary".into(),
                        "string".into(),
                        self.span()?,
                    ))
                }
            }),
            x => Err(ShellError::CantConvert(
                "string".into(),
                x.get_type().to_string(),
                self.span()?,
            )),
        }
    }

    pub fn as_spanned_string(&self) -> Result<Spanned<String>, ShellError> {
        match self {
            Value::String { val, span } => Ok(Spanned {
                item: val.to_string(),
                span: *span,
            }),
            Value::Binary { val, span } => Ok(match std::str::from_utf8(val) {
                Ok(s) => Spanned {
                    item: s.to_string(),
                    span: *span,
                },
                Err(_) => {
                    return Err(ShellError::CantConvert(
                        "binary".into(),
                        "string".into(),
                        self.span()?,
                    ))
                }
            }),
            x => Err(ShellError::CantConvert(
                "string".into(),
                x.get_type().to_string(),
                self.span()?,
            )),
        }
    }

    pub fn as_path(&self) -> Result<PathBuf, ShellError> {
        match self {
            Value::String { val, .. } => Ok(PathBuf::from(val)),
            x => Err(ShellError::CantConvert(
                "path".into(),
                x.get_type().to_string(),
                self.span()?,
            )),
        }
    }

    pub fn as_block(&self) -> Result<BlockId, ShellError> {
        match self {
            Value::Block { val, .. } => Ok(*val),
            x => Err(ShellError::CantConvert(
                "block".into(),
                x.get_type().to_string(),
                self.span()?,
            )),
        }
    }

    pub fn as_record(&self) -> Result<(&[String], &[Value]), ShellError> {
        match self {
            Value::Record { cols, vals, .. } => Ok((cols, vals)),
            x => Err(ShellError::CantConvert(
                "record".into(),
                x.get_type().to_string(),
                self.span()?,
            )),
        }
    }

    pub fn as_list(&self) -> Result<&[Value], ShellError> {
        match self {
            Value::List { vals, .. } => Ok(vals),
            x => Err(ShellError::CantConvert(
                "list".into(),
                x.get_type().to_string(),
                self.span()?,
            )),
        }
    }

    pub fn as_bool(&self) -> Result<bool, ShellError> {
        match self {
            Value::Bool { val, .. } => Ok(*val),
            x => Err(ShellError::CantConvert(
                "boolean".into(),
                x.get_type().to_string(),
                self.span()?,
            )),
        }
    }

    pub fn as_float(&self) -> Result<f64, ShellError> {
        match self {
            Value::Float { val, .. } => Ok(*val),
            x => Err(ShellError::CantConvert(
                "float".into(),
                x.get_type().to_string(),
                self.span()?,
            )),
        }
    }

    pub fn as_integer(&self) -> Result<i64, ShellError> {
        match self {
            Value::Int { val, .. } => Ok(*val),
            x => Err(ShellError::CantConvert(
                "float".into(),
                x.get_type().to_string(),
                self.span()?,
            )),
        }
    }

    /// Get the span for the current value
    pub fn span(&self) -> Result<Span, ShellError> {
        match self {
            Value::Error { error } => Err(error.clone()),
            Value::Bool { span, .. } => Ok(*span),
            Value::Int { span, .. } => Ok(*span),
            Value::Float { span, .. } => Ok(*span),
            Value::Filesize { span, .. } => Ok(*span),
            Value::Duration { span, .. } => Ok(*span),
            Value::Date { span, .. } => Ok(*span),
            Value::Range { span, .. } => Ok(*span),
            Value::String { span, .. } => Ok(*span),
            Value::Record { span, .. } => Ok(*span),
            Value::List { span, .. } => Ok(*span),
            Value::Block { span, .. } => Ok(*span),
            Value::Nothing { span, .. } => Ok(*span),
            Value::Binary { span, .. } => Ok(*span),
            Value::CellPath { span, .. } => Ok(*span),
            Value::CustomValue { span, .. } => Ok(*span),
        }
    }

    /// Update the value with a new span
    pub fn with_span(mut self, new_span: Span) -> Value {
        match &mut self {
            Value::Bool { span, .. } => *span = new_span,
            Value::Int { span, .. } => *span = new_span,
            Value::Float { span, .. } => *span = new_span,
            Value::Filesize { span, .. } => *span = new_span,
            Value::Duration { span, .. } => *span = new_span,
            Value::Date { span, .. } => *span = new_span,
            Value::Range { span, .. } => *span = new_span,
            Value::String { span, .. } => *span = new_span,
            Value::Record { span, .. } => *span = new_span,
            Value::List { span, .. } => *span = new_span,
            Value::Block { span, .. } => *span = new_span,
            Value::Nothing { span, .. } => *span = new_span,
            Value::Error { .. } => {}
            Value::Binary { span, .. } => *span = new_span,
            Value::CellPath { span, .. } => *span = new_span,
            Value::CustomValue { span, .. } => *span = new_span,
        }

        self
    }

    /// Get the type of the current Value
    pub fn get_type(&self) -> Type {
        match self {
            Value::Bool { .. } => Type::Bool,
            Value::Int { .. } => Type::Int,
            Value::Float { .. } => Type::Float,
            Value::Filesize { .. } => Type::Filesize,
            Value::Duration { .. } => Type::Duration,
            Value::Date { .. } => Type::Date,
            Value::Range { .. } => Type::Range,
            Value::String { .. } => Type::String,
            Value::Record { cols, vals, .. } => Type::Record(
                cols.iter()
                    .zip(vals.iter())
                    .map(|(x, y)| (x.clone(), y.get_type()))
                    .collect(),
            ),
            Value::List { .. } => Type::List(Box::new(Type::Unknown)), // FIXME
            Value::Nothing { .. } => Type::Nothing,
            Value::Block { .. } => Type::Block,
            Value::Error { .. } => Type::Error,
            Value::Binary { .. } => Type::Binary,
            Value::CellPath { .. } => Type::CellPath,
            Value::CustomValue { .. } => Type::Custom,
        }
    }

    pub fn get_data_by_key(&self, name: &str) -> Option<Value> {
        match self {
            Value::Record { cols, vals, .. } => cols
                .iter()
                .zip(vals.iter())
                .find(|(col, _)| col == &name)
                .map(|(_, val)| val.clone()),
            Value::List { vals, span } => {
                let mut out = vec![];
                for item in vals {
                    match item {
                        Value::Record { .. } => match item.get_data_by_key(name) {
                            Some(v) => out.push(v),
                            None => out.push(Value::nothing(*span)),
                        },
                        _ => out.push(Value::nothing(*span)),
                    }
                }

                if !out.is_empty() {
                    Some(Value::List {
                        vals: out,
                        span: *span,
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Convert Value into string. Note that Streams will be consumed.
    pub fn into_string(&self, separator: &str, config: &Config) -> String {
        match self {
            Value::Bool { val, .. } => val.to_string(),
            Value::Int { val, .. } => val.to_string(),
            Value::Float { val, .. } => val.to_string(),
            Value::Filesize { val, .. } => format_filesize(*val, config),
            Value::Duration { val, .. } => format_duration(*val),
            Value::Date { val, .. } => HumanTime::from(*val).to_string(),
            Value::Range { val, .. } => {
                format!(
                    "{}..{}",
                    val.from.into_string(", ", config),
                    val.to.into_string(", ", config)
                )
            }
            Value::String { val, .. } => val.clone(),
            Value::List { vals: val, .. } => format!(
                "[{}]",
                val.iter()
                    .map(|x| x.into_string(", ", config))
                    .collect::<Vec<_>>()
                    .join(separator)
            ),
            Value::Record { cols, vals, .. } => format!(
                "{{{}}}",
                cols.iter()
                    .zip(vals.iter())
                    .map(|(x, y)| format!("{}: {}", x, y.into_string(", ", config)))
                    .collect::<Vec<_>>()
                    .join(separator)
            ),
            Value::Block { val, .. } => format!("<Block {}>", val),
            Value::Nothing { .. } => String::new(),
            Value::Error { error } => format!("{:?}", error),
            Value::Binary { val, .. } => format!("{:?}", val),
            Value::CellPath { val, .. } => val.into_string(),
            Value::CustomValue { val, .. } => val.value_string(),
        }
    }

    /// Convert Value into string. Note that Streams will be consumed.
    pub fn into_abbreviated_string(&self, config: &Config) -> String {
        match self {
            Value::Bool { val, .. } => val.to_string(),
            Value::Int { val, .. } => val.to_string(),
            Value::Float { val, .. } => val.to_string(),
            Value::Filesize { val, .. } => format_filesize(*val, config),
            Value::Duration { val, .. } => format_duration(*val),
            Value::Date { val, .. } => HumanTime::from(*val).to_string(),
            Value::Range { val, .. } => {
                format!(
                    "{}..{}",
                    val.from.into_string(", ", config),
                    val.to.into_string(", ", config)
                )
            }
            Value::String { val, .. } => val.to_string(),
            Value::List { ref vals, .. } => match &vals[..] {
                [Value::Record { .. }, _end @ ..] => format!(
                    "[table {} row{}]",
                    vals.len(),
                    if vals.len() == 1 { "" } else { "s" }
                ),
                _ => format!(
                    "[list {} item{}]",
                    vals.len(),
                    if vals.len() == 1 { "" } else { "s" }
                ),
            },
            Value::Record { cols, .. } => format!(
                "{{record {} field{}}}",
                cols.len(),
                if cols.len() == 1 { "" } else { "s" }
            ),
            Value::Block { val, .. } => format!("<Block {}>", val),
            Value::Nothing { .. } => String::new(),
            Value::Error { error } => format!("{:?}", error),
            Value::Binary { val, .. } => format!("{:?}", val),
            Value::CellPath { val, .. } => val.into_string(),
            Value::CustomValue { val, .. } => val.value_string(),
        }
    }

    /// Convert Value into a debug string
    pub fn debug_value(&self) -> String {
        format!("{:#?}", self)
    }

    /// Convert Value into string. Note that Streams will be consumed.
    pub fn debug_string(&self, separator: &str, config: &Config) -> String {
        match self {
            Value::Bool { val, .. } => val.to_string(),
            Value::Int { val, .. } => val.to_string(),
            Value::Float { val, .. } => val.to_string(),
            Value::Filesize { val, .. } => format_filesize(*val, config),
            Value::Duration { val, .. } => format_duration(*val),
            Value::Date { val, .. } => format!("{:?}", val),
            Value::Range { val, .. } => {
                format!(
                    "{}..{}",
                    val.from.into_string(", ", config),
                    val.to.into_string(", ", config)
                )
            }
            Value::String { val, .. } => val.clone(),
            Value::List { vals: val, .. } => format!(
                "[{}]",
                val.iter()
                    .map(|x| x.into_string(", ", config))
                    .collect::<Vec<_>>()
                    .join(separator)
            ),
            Value::Record { cols, vals, .. } => format!(
                "{{{}}}",
                cols.iter()
                    .zip(vals.iter())
                    .map(|(x, y)| format!("{}: {}", x, y.into_string(", ", config)))
                    .collect::<Vec<_>>()
                    .join(separator)
            ),
            Value::Block { val, .. } => format!("<Block {}>", val),
            Value::Nothing { .. } => String::new(),
            Value::Error { error } => format!("{:?}", error),
            Value::Binary { val, .. } => format!("{:?}", val),
            Value::CellPath { val, .. } => val.into_string(),
            Value::CustomValue { val, .. } => val.value_string(),
        }
    }

    /// Check if the content is empty
    pub fn is_empty(&self) -> bool {
        match self {
            Value::String { val, .. } => val.is_empty(),
            Value::List { vals, .. } => {
                vals.is_empty() && vals.iter().all(|v| v.clone().is_empty())
            }
            Value::Record { cols, vals, .. } => {
                cols.iter().all(|v| v.is_empty()) && vals.iter().all(|v| v.clone().is_empty())
            }
            Value::Nothing { .. } => true,
            _ => false,
        }
    }

    /// Create a new `Nothing` value
    pub fn nothing(span: Span) -> Value {
        Value::Nothing { span }
    }

    /// Follow a given column path into the value: for example accessing nth elements in a stream or list
    pub fn follow_cell_path(self, cell_path: &[PathMember]) -> Result<Value, ShellError> {
        let mut current = self;
        for member in cell_path {
            // FIXME: this uses a few extra clones for simplicity, but there may be a way
            // to traverse the path without them
            match member {
                PathMember::Int {
                    val: count,
                    span: origin_span,
                } => {
                    // Treat a numeric path member as `nth <val>`
                    match &mut current {
                        Value::List { vals: val, .. } => {
                            if let Some(item) = val.get(*count) {
                                current = item.clone();
                            } else {
                                return Err(ShellError::AccessBeyondEnd(val.len(), *origin_span));
                            }
                        }
                        Value::Binary { val, .. } => {
                            if let Some(item) = val.get(*count) {
                                current = Value::Int {
                                    val: *item as i64,
                                    span: *origin_span,
                                };
                            } else {
                                return Err(ShellError::AccessBeyondEnd(val.len(), *origin_span));
                            }
                        }
                        Value::Range { val, .. } => {
                            if let Some(item) = val.clone().into_range_iter()?.nth(*count) {
                                current = item.clone();
                            } else {
                                return Err(ShellError::AccessBeyondEndOfStream(*origin_span));
                            }
                        }
                        Value::CustomValue { val, .. } => {
                            current = val.follow_path_int(*count, *origin_span)?;
                        }
                        x => {
                            return Err(ShellError::IncompatiblePathAccess(
                                format!("{}", x.get_type()),
                                *origin_span,
                            ))
                        }
                    }
                }
                PathMember::String {
                    val: column_name,
                    span: origin_span,
                } => match &mut current {
                    Value::Record { cols, vals, span } => {
                        let cols = cols.clone();
                        let span = *span;

                        if let Some(found) =
                            cols.iter().zip(vals.iter()).find(|x| x.0 == column_name)
                        {
                            current = found.1.clone();
                        } else if let Some(suggestion) = did_you_mean(&cols, column_name) {
                            return Err(ShellError::DidYouMean(suggestion, *origin_span));
                        } else {
                            return Err(ShellError::CantFindColumn(*origin_span, span));
                        }
                    }
                    Value::List { vals, span } => {
                        let mut output = vec![];
                        for val in vals {
                            output.push(val.clone().follow_cell_path(&[PathMember::String {
                                val: column_name.clone(),
                                span: *origin_span,
                            }])?);
                            // if let Value::Record { cols, vals, .. } = val {
                            //     for col in cols.iter().enumerate() {
                            //         if col.1 == column_name {
                            //             output.push(vals[col.0].clone());
                            //         }
                            //     }
                            // }
                        }

                        current = Value::List {
                            vals: output,
                            span: *span,
                        };
                    }
                    Value::CustomValue { val, .. } => {
                        current = val.follow_path_string(column_name.clone(), *origin_span)?;
                    }
                    x => {
                        return Err(ShellError::IncompatiblePathAccess(
                            format!("{}", x.get_type()),
                            *origin_span,
                        ))
                    }
                },
            }
        }

        Ok(current)
    }

    /// Follow a given column path into the value: for example accessing nth elements in a stream or list
    pub fn update_cell_path(
        &mut self,
        cell_path: &[PathMember],
        callback: Box<dyn FnOnce(&Value) -> Value>,
    ) -> Result<(), ShellError> {
        let orig = self.clone();

        let new_val = callback(&orig.follow_cell_path(cell_path)?);

        match new_val {
            Value::Error { error } => Err(error),
            new_val => self.replace_data_at_cell_path(cell_path, new_val),
        }
    }

    pub fn replace_data_at_cell_path(
        &mut self,
        cell_path: &[PathMember],
        new_val: Value,
    ) -> Result<(), ShellError> {
        match cell_path.first() {
            Some(path_member) => match path_member {
                PathMember::String {
                    val: col_name,
                    span,
                } => match self {
                    Value::List { vals, .. } => {
                        for val in vals.iter_mut() {
                            match val {
                                Value::Record { cols, vals, .. } => {
                                    let mut found = false;
                                    for col in cols.iter().zip(vals.iter_mut()) {
                                        if col.0 == col_name {
                                            found = true;
                                            col.1.replace_data_at_cell_path(
                                                &cell_path[1..],
                                                new_val.clone(),
                                            )?
                                        }
                                    }
                                    if !found {
                                        if cell_path.len() == 1 {
                                            cols.push(col_name.clone());
                                            vals.push(new_val);
                                            break;
                                        } else {
                                            let mut new_col = Value::Record {
                                                cols: vec![],
                                                vals: vec![],
                                                span: new_val.span()?,
                                            };
                                            new_col.replace_data_at_cell_path(
                                                &cell_path[1..],
                                                new_val,
                                            )?;
                                            vals.push(new_col);
                                            break;
                                        }
                                    }
                                }
                                v => return Err(ShellError::CantFindColumn(*span, v.span()?)),
                            }
                        }
                    }
                    Value::Record { cols, vals, .. } => {
                        let mut found = false;

                        for col in cols.iter().zip(vals.iter_mut()) {
                            if col.0 == col_name {
                                found = true;

                                col.1
                                    .replace_data_at_cell_path(&cell_path[1..], new_val.clone())?
                            }
                        }
                        if !found {
                            if cell_path.len() == 1 {
                                cols.push(col_name.clone());
                                vals.push(new_val);
                            } else {
                                let mut new_col = Value::Record {
                                    cols: vec![],
                                    vals: vec![],
                                    span: new_val.span()?,
                                };
                                new_col.replace_data_at_cell_path(&cell_path[1..], new_val)?;
                                vals.push(new_col);
                            }
                        }
                    }
                    v => return Err(ShellError::CantFindColumn(*span, v.span()?)),
                },
                PathMember::Int { val: row_num, span } => match self {
                    Value::List { vals, .. } => {
                        if let Some(v) = vals.get_mut(*row_num) {
                            v.replace_data_at_cell_path(&cell_path[1..], new_val)?
                        } else {
                            return Err(ShellError::AccessBeyondEnd(vals.len(), *span));
                        }
                    }
                    v => return Err(ShellError::NotAList(*span, v.span()?)),
                },
            },
            None => {
                *self = new_val;
            }
        }
        Ok(())
    }

    pub fn is_true(&self) -> bool {
        matches!(self, Value::Bool { val: true, .. })
    }

    pub fn columns(&self) -> Vec<String> {
        match self {
            Value::Record { cols, .. } => cols.clone(),
            _ => vec![],
        }
    }

    pub fn string(val: impl Into<String>, span: Span) -> Value {
        Value::String {
            val: val.into(),
            span,
        }
    }

    pub fn int(val: i64, span: Span) -> Value {
        Value::Int { val, span }
    }

    pub fn float(val: f64, span: Span) -> Value {
        Value::Float { val, span }
    }

    pub fn boolean(val: bool, span: Span) -> Value {
        Value::Bool { val, span }
    }

    /// Note: Only use this for test data, *not* live data, as it will point into unknown source
    /// when used in errors.
    pub fn test_string(s: impl Into<String>) -> Value {
        Value::String {
            val: s.into(),
            span: Span::test_data(),
        }
    }

    /// Note: Only use this for test data, *not* live data, as it will point into unknown source
    /// when used in errors.
    pub fn test_int(val: i64) -> Value {
        Value::Int {
            val,
            span: Span::test_data(),
        }
    }

    /// Note: Only use this for test data, *not* live data, as it will point into unknown source
    /// when used in errors.
    pub fn test_float(val: f64) -> Value {
        Value::Float {
            val,
            span: Span::test_data(),
        }
    }

    /// Note: Only use this for test data, *not* live data, as it will point into unknown source
    /// when used in errors.
    pub fn test_bool(val: bool) -> Value {
        Value::Bool {
            val,
            span: Span::test_data(),
        }
    }

    /// Note: Only use this for test data, *not* live data, as it will point into unknown source
    /// when used in errors.
    pub fn test_filesize(val: i64) -> Value {
        Value::Filesize {
            val,
            span: Span::test_data(),
        }
    }

    /// Note: Only use this for test data, *not* live data, as it will point into unknown source
    /// when used in errors.
    pub fn test_nothing() -> Value {
        Value::Nothing {
            span: Span::test_data(),
        }
    }

    /// Note: Only use this for test data, *not* live data, as it will point into unknown source
    /// when used in errors.
    pub fn test_record(cols: Vec<impl Into<String>>, vals: Vec<Value>) -> Value {
        Value::Record {
            cols: cols.into_iter().map(|s| s.into()).collect(),
            vals,

            span: Span::test_data(),
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::Nothing {
            span: Span { start: 0, end: 0 },
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Compare two floating point numbers. The decision interval for equality is dynamically
        // scaled as the value being compared increases in magnitude.
        fn compare_floats(val: f64, other: f64) -> Option<Ordering> {
            let prec = f64::EPSILON.max(val.abs() * f64::EPSILON);

            if (other - val).abs() < prec {
                return Some(Ordering::Equal);
            }

            val.partial_cmp(&other)
        }

        match (self, other) {
            (Value::Bool { val: lhs, .. }, Value::Bool { val: rhs, .. }) => lhs.partial_cmp(rhs),
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => lhs.partial_cmp(rhs),
            (Value::Float { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                compare_floats(*lhs, *rhs)
            }
            (Value::Date { val: lhs, .. }, Value::Date { val: rhs, .. }) => lhs.partial_cmp(rhs),
            (Value::String { val: lhs, .. }, Value::String { val: rhs, .. }) => {
                lhs.partial_cmp(rhs)
            }
            (Value::Int { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                compare_floats(*lhs as f64, *rhs)
            }
            (Value::Float { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                compare_floats(*lhs, *rhs as f64)
            }
            (Value::Duration { val: lhs, .. }, Value::Duration { val: rhs, .. }) => {
                lhs.partial_cmp(rhs)
            }
            (Value::Filesize { val: lhs, .. }, Value::Filesize { val: rhs, .. }) => {
                lhs.partial_cmp(rhs)
            }
            (Value::Block { val: b1, .. }, Value::Block { val: b2, .. }) if b1 == b2 => {
                Some(Ordering::Equal)
            }
            (Value::List { vals: lhs, .. }, Value::List { vals: rhs, .. }) => lhs.partial_cmp(rhs),
            (
                Value::Record {
                    vals: lhs,
                    cols: lhs_headers,
                    ..
                },
                Value::Record {
                    vals: rhs,
                    cols: rhs_headers,
                    ..
                },
            ) if lhs_headers == rhs_headers && lhs == rhs => Some(Ordering::Equal),
            (Value::Binary { val: lhs, .. }, Value::Binary { val: rhs, .. }) => {
                lhs.partial_cmp(rhs)
            }
            (Value::CustomValue { val: lhs, .. }, rhs) => lhs.partial_cmp(rhs),
            (Value::Nothing { .. }, Value::Nothing { .. }) => Some(Ordering::Equal),
            (_, _) => None,
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        self.partial_cmp(other).map_or(false, Ordering::is_eq)
    }
}

impl Value {
    pub fn add(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span()?, rhs.span()?]);

        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if let Some(val) = lhs.checked_add(*rhs) {
                    Ok(Value::Int { val, span })
                } else {
                    Err(ShellError::OperatorOverflow(
                        "add operation overflowed".into(),
                        span,
                    ))
                }
            }
            (Value::Int { val: lhs, .. }, Value::Float { val: rhs, .. }) => Ok(Value::Float {
                val: *lhs as f64 + *rhs,
                span,
            }),
            (Value::Float { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Float {
                val: *lhs + *rhs as f64,
                span,
            }),
            (Value::Float { val: lhs, .. }, Value::Float { val: rhs, .. }) => Ok(Value::Float {
                val: lhs + rhs,
                span,
            }),
            (Value::String { val: lhs, .. }, Value::String { val: rhs, .. }) => Ok(Value::String {
                val: lhs.to_string() + rhs,
                span,
            }),
            (Value::Date { val: lhs, .. }, Value::Duration { val: rhs, .. }) => {
                match lhs.checked_add_signed(chrono::Duration::nanoseconds(*rhs)) {
                    Some(val) => Ok(Value::Date { val, span }),
                    _ => Err(ShellError::OperatorOverflow(
                        "addition operation overflowed".into(),
                        span,
                    )),
                }
            }
            (Value::Duration { val: lhs, .. }, Value::Duration { val: rhs, .. }) => {
                if let Some(val) = lhs.checked_add(*rhs) {
                    Ok(Value::Duration { val, span })
                } else {
                    Err(ShellError::OperatorOverflow(
                        "add operation overflowed".into(),
                        span,
                    ))
                }
            }
            (Value::Filesize { val: lhs, .. }, Value::Filesize { val: rhs, .. }) => {
                if let Some(val) = lhs.checked_add(*rhs) {
                    Ok(Value::Filesize { val, span })
                } else {
                    Err(ShellError::OperatorOverflow(
                        "add operation overflowed".into(),
                        span,
                    ))
                }
            }

            (Value::CustomValue { val: lhs, span }, rhs) => {
                lhs.operation(*span, Operator::Plus, op, rhs)
            }

            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span()?,
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span()?,
            }),
        }
    }
    pub fn sub(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span()?, rhs.span()?]);

        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if let Some(val) = lhs.checked_sub(*rhs) {
                    Ok(Value::Int { val, span })
                } else {
                    Err(ShellError::OperatorOverflow(
                        "subtraction operation overflowed".into(),
                        span,
                    ))
                }
            }
            (Value::Int { val: lhs, .. }, Value::Float { val: rhs, .. }) => Ok(Value::Float {
                val: *lhs as f64 - *rhs,
                span,
            }),
            (Value::Float { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Float {
                val: *lhs - *rhs as f64,
                span,
            }),
            (Value::Float { val: lhs, .. }, Value::Float { val: rhs, .. }) => Ok(Value::Float {
                val: lhs - rhs,
                span,
            }),
            (Value::Date { val: lhs, .. }, Value::Duration { val: rhs, .. }) => {
                match lhs.checked_sub_signed(chrono::Duration::nanoseconds(*rhs)) {
                    Some(val) => Ok(Value::Date { val, span }),
                    _ => Err(ShellError::OperatorOverflow(
                        "subtraction operation overflowed".into(),
                        span,
                    )),
                }
            }
            (Value::Duration { val: lhs, .. }, Value::Duration { val: rhs, .. }) => {
                if let Some(val) = lhs.checked_sub(*rhs) {
                    Ok(Value::Duration { val, span })
                } else {
                    Err(ShellError::OperatorOverflow(
                        "subtraction operation overflowed".into(),
                        span,
                    ))
                }
            }
            (Value::Filesize { val: lhs, .. }, Value::Filesize { val: rhs, .. }) => {
                if let Some(val) = lhs.checked_sub(*rhs) {
                    Ok(Value::Filesize { val, span })
                } else {
                    Err(ShellError::OperatorOverflow(
                        "add operation overflowed".into(),
                        span,
                    ))
                }
            }

            (Value::CustomValue { val: lhs, span }, rhs) => {
                lhs.operation(*span, Operator::Minus, op, rhs)
            }

            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span()?,
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span()?,
            }),
        }
    }
    pub fn mul(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span()?, rhs.span()?]);

        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if let Some(val) = lhs.checked_mul(*rhs) {
                    Ok(Value::Int { val, span })
                } else {
                    Err(ShellError::OperatorOverflow(
                        "multiply operation overflowed".into(),
                        span,
                    ))
                }
            }
            (Value::Int { val: lhs, .. }, Value::Float { val: rhs, .. }) => Ok(Value::Float {
                val: *lhs as f64 * *rhs,
                span,
            }),
            (Value::Float { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Float {
                val: *lhs * *rhs as f64,
                span,
            }),
            (Value::Float { val: lhs, .. }, Value::Float { val: rhs, .. }) => Ok(Value::Float {
                val: lhs * rhs,
                span,
            }),
            (Value::CustomValue { val: lhs, span }, rhs) => {
                lhs.operation(*span, Operator::Multiply, op, rhs)
            }

            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span()?,
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span()?,
            }),
        }
    }
    pub fn div(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span()?, rhs.span()?]);

        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if *rhs != 0 {
                    if lhs % rhs == 0 {
                        Ok(Value::Int {
                            val: lhs / rhs,
                            span,
                        })
                    } else {
                        Ok(Value::Float {
                            val: (*lhs as f64) / (*rhs as f64),
                            span,
                        })
                    }
                } else {
                    Err(ShellError::DivisionByZero(op))
                }
            }
            (Value::Int { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                if *rhs != 0.0 {
                    Ok(Value::Float {
                        val: *lhs as f64 / *rhs,
                        span,
                    })
                } else {
                    Err(ShellError::DivisionByZero(op))
                }
            }
            (Value::Float { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if *rhs != 0 {
                    Ok(Value::Float {
                        val: *lhs / *rhs as f64,
                        span,
                    })
                } else {
                    Err(ShellError::DivisionByZero(op))
                }
            }
            (Value::Float { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                if *rhs != 0.0 {
                    Ok(Value::Float {
                        val: lhs / rhs,
                        span,
                    })
                } else {
                    Err(ShellError::DivisionByZero(op))
                }
            }
            (Value::Filesize { val: lhs, .. }, Value::Filesize { val: rhs, .. }) => {
                if *rhs != 0 {
                    if lhs % rhs == 0 {
                        Ok(Value::Int {
                            val: lhs / rhs,
                            span,
                        })
                    } else {
                        Ok(Value::Float {
                            val: (*lhs as f64) / (*rhs as f64),
                            span,
                        })
                    }
                } else {
                    Err(ShellError::DivisionByZero(op))
                }
            }
            (Value::Duration { val: lhs, .. }, Value::Duration { val: rhs, .. }) => {
                if *rhs != 0 {
                    if lhs % rhs == 0 {
                        Ok(Value::Int {
                            val: lhs / rhs,
                            span,
                        })
                    } else {
                        Ok(Value::Float {
                            val: (*lhs as f64) / (*rhs as f64),
                            span,
                        })
                    }
                } else {
                    Err(ShellError::DivisionByZero(op))
                }
            }
            (Value::CustomValue { val: lhs, span }, rhs) => {
                lhs.operation(*span, Operator::Divide, op, rhs)
            }

            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span()?,
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span()?,
            }),
        }
    }
    pub fn lt(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span()?, rhs.span()?]);

        if let (Value::CustomValue { val: lhs, span }, rhs) = (self, rhs) {
            return lhs.operation(*span, Operator::LessThan, op, rhs);
        }

        match self.partial_cmp(rhs) {
            Some(ordering) => Ok(Value::Bool {
                val: matches!(ordering, Ordering::Less),
                span,
            }),
            None => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span()?,
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span()?,
            }),
        }
    }
    pub fn lte(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span()?, rhs.span()?]);

        if let (Value::CustomValue { val: lhs, span }, rhs) = (self, rhs) {
            return lhs.operation(*span, Operator::LessThanOrEqual, op, rhs);
        }

        match self.partial_cmp(rhs) {
            Some(ordering) => Ok(Value::Bool {
                val: matches!(ordering, Ordering::Less | Ordering::Equal),
                span,
            }),
            None => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span()?,
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span()?,
            }),
        }
    }
    pub fn gt(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span()?, rhs.span()?]);

        if let (Value::CustomValue { val: lhs, span }, rhs) = (self, rhs) {
            return lhs.operation(*span, Operator::GreaterThan, op, rhs);
        }

        match self.partial_cmp(rhs) {
            Some(ordering) => Ok(Value::Bool {
                val: matches!(ordering, Ordering::Greater),
                span,
            }),
            None => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span()?,
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span()?,
            }),
        }
    }
    pub fn gte(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span()?, rhs.span()?]);

        if let (Value::CustomValue { val: lhs, span }, rhs) = (self, rhs) {
            return lhs.operation(*span, Operator::GreaterThanOrEqual, op, rhs);
        }

        match self.partial_cmp(rhs) {
            Some(ordering) => Ok(Value::Bool {
                val: matches!(ordering, Ordering::Greater | Ordering::Equal),
                span,
            }),
            None => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span()?,
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span()?,
            }),
        }
    }
    pub fn eq(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span()?, rhs.span()?]);

        if let (Value::CustomValue { val: lhs, span }, rhs) = (self, rhs) {
            return lhs.operation(*span, Operator::Equal, op, rhs);
        }

        match self.partial_cmp(rhs) {
            Some(ordering) => Ok(Value::Bool {
                val: matches!(ordering, Ordering::Equal),
                span,
            }),
            None => match (self, rhs) {
                (Value::Nothing { .. }, _) | (_, Value::Nothing { .. }) => {
                    Ok(Value::Bool { val: false, span })
                }
                _ => Err(ShellError::OperatorMismatch {
                    op_span: op,
                    lhs_ty: self.get_type(),
                    lhs_span: self.span()?,
                    rhs_ty: rhs.get_type(),
                    rhs_span: rhs.span()?,
                }),
            },
        }
    }
    pub fn ne(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span()?, rhs.span()?]);

        if let (Value::CustomValue { val: lhs, span }, rhs) = (self, rhs) {
            return lhs.operation(*span, Operator::NotEqual, op, rhs);
        }

        match self.partial_cmp(rhs) {
            Some(ordering) => Ok(Value::Bool {
                val: !matches!(ordering, Ordering::Equal),
                span,
            }),
            None => match (self, rhs) {
                (Value::Nothing { .. }, _) | (_, Value::Nothing { .. }) => {
                    Ok(Value::Bool { val: true, span })
                }
                _ => Err(ShellError::OperatorMismatch {
                    op_span: op,
                    lhs_ty: self.get_type(),
                    lhs_span: self.span()?,
                    rhs_ty: rhs.get_type(),
                    rhs_span: rhs.span()?,
                }),
            },
        }
    }

    pub fn r#in(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span()?, rhs.span()?]);

        match (self, rhs) {
            (lhs, Value::Range { val: rhs, .. }) => Ok(Value::Bool {
                val: rhs.contains(lhs),
                span,
            }),
            (Value::String { val: lhs, .. }, Value::String { val: rhs, .. }) => Ok(Value::Bool {
                val: rhs.contains(lhs),
                span,
            }),
            (lhs, Value::List { vals: rhs, .. }) => Ok(Value::Bool {
                val: rhs.contains(lhs),
                span,
            }),
            (Value::String { val: lhs, .. }, Value::Record { cols: rhs, .. }) => Ok(Value::Bool {
                val: rhs.contains(lhs),
                span,
            }),
            (Value::String { .. } | Value::Int { .. }, Value::CellPath { val: rhs, .. }) => {
                let val = rhs.members.iter().any(|member| match (self, member) {
                    (Value::Int { val: lhs, .. }, PathMember::Int { val: rhs, .. }) => {
                        *lhs == *rhs as i64
                    }
                    (Value::String { val: lhs, .. }, PathMember::String { val: rhs, .. }) => {
                        lhs == rhs
                    }
                    (Value::String { .. }, PathMember::Int { .. })
                    | (Value::Int { .. }, PathMember::String { .. }) => false,
                    _ => unreachable!(
                        "outer match arm ensures `self` is either a `String` or `Int` variant"
                    ),
                });

                Ok(Value::Bool { val, span })
            }
            (Value::CellPath { val: lhs, .. }, Value::CellPath { val: rhs, .. }) => {
                Ok(Value::Bool {
                    val: rhs
                        .members
                        .windows(lhs.members.len())
                        .any(|member_window| member_window == rhs.members),
                    span,
                })
            }
            (Value::CustomValue { val: lhs, span }, rhs) => {
                lhs.operation(*span, Operator::In, op, rhs)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span()?,
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span()?,
            }),
        }
    }

    pub fn not_in(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span()?, rhs.span()?]);

        match (self, rhs) {
            (lhs, Value::Range { val: rhs, .. }) => Ok(Value::Bool {
                val: !rhs.contains(lhs),
                span,
            }),
            (Value::String { val: lhs, .. }, Value::String { val: rhs, .. }) => Ok(Value::Bool {
                val: !rhs.contains(lhs),
                span,
            }),
            (lhs, Value::List { vals: rhs, .. }) => Ok(Value::Bool {
                val: !rhs.contains(lhs),
                span,
            }),
            (Value::String { val: lhs, .. }, Value::Record { cols: rhs, .. }) => Ok(Value::Bool {
                val: !rhs.contains(lhs),
                span,
            }),
            (Value::String { .. } | Value::Int { .. }, Value::CellPath { val: rhs, .. }) => {
                let val = rhs.members.iter().any(|member| match (self, member) {
                    (Value::Int { val: lhs, .. }, PathMember::Int { val: rhs, .. }) => {
                        *lhs != *rhs as i64
                    }
                    (Value::String { val: lhs, .. }, PathMember::String { val: rhs, .. }) => {
                        lhs != rhs
                    }
                    (Value::String { .. }, PathMember::Int { .. })
                    | (Value::Int { .. }, PathMember::String { .. }) => true,
                    _ => unreachable!(
                        "outer match arm ensures `self` is either a `String` or `Int` variant"
                    ),
                });

                Ok(Value::Bool { val, span })
            }
            (Value::CellPath { val: lhs, .. }, Value::CellPath { val: rhs, .. }) => {
                Ok(Value::Bool {
                    val: rhs
                        .members
                        .windows(lhs.members.len())
                        .all(|member_window| member_window != rhs.members),
                    span,
                })
            }
            (Value::CustomValue { val: lhs, span }, rhs) => {
                lhs.operation(*span, Operator::NotIn, op, rhs)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span()?,
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span()?,
            }),
        }
    }

    pub fn contains(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span()?, rhs.span()?]);

        match (self, rhs) {
            (Value::String { val: lhs, .. }, Value::String { val: rhs, .. }) => Ok(Value::Bool {
                val: lhs.contains(rhs),
                span,
            }),
            (Value::CustomValue { val: lhs, span }, rhs) => {
                lhs.operation(*span, Operator::Contains, op, rhs)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span()?,
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span()?,
            }),
        }
    }

    pub fn not_contains(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span()?, rhs.span()?]);

        match (self, rhs) {
            (Value::String { val: lhs, .. }, Value::String { val: rhs, .. }) => Ok(Value::Bool {
                val: !lhs.contains(rhs),
                span,
            }),
            (Value::CustomValue { val: lhs, span }, rhs) => {
                lhs.operation(*span, Operator::NotContains, op, rhs)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span()?,
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span()?,
            }),
        }
    }

    pub fn modulo(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span()?, rhs.span()?]);

        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if *rhs != 0 {
                    Ok(Value::Int {
                        val: lhs % rhs,
                        span,
                    })
                } else {
                    Err(ShellError::DivisionByZero(op))
                }
            }
            (Value::Int { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                if *rhs != 0.0 {
                    Ok(Value::Float {
                        val: *lhs as f64 % *rhs,
                        span,
                    })
                } else {
                    Err(ShellError::DivisionByZero(op))
                }
            }
            (Value::Float { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if *rhs != 0 {
                    Ok(Value::Float {
                        val: *lhs % *rhs as f64,
                        span,
                    })
                } else {
                    Err(ShellError::DivisionByZero(op))
                }
            }
            (Value::Float { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                if *rhs != 0.0 {
                    Ok(Value::Float {
                        val: lhs % rhs,
                        span,
                    })
                } else {
                    Err(ShellError::DivisionByZero(op))
                }
            }
            (Value::CustomValue { val: lhs, span }, rhs) => {
                lhs.operation(*span, Operator::Modulo, op, rhs)
            }

            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span()?,
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span()?,
            }),
        }
    }

    pub fn and(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span()?, rhs.span()?]);

        match (self, rhs) {
            (Value::Bool { val: lhs, .. }, Value::Bool { val: rhs, .. }) => Ok(Value::Bool {
                val: *lhs && *rhs,
                span,
            }),
            (Value::CustomValue { val: lhs, span }, rhs) => {
                lhs.operation(*span, Operator::And, op, rhs)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span()?,
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span()?,
            }),
        }
    }

    pub fn or(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span()?, rhs.span()?]);

        match (self, rhs) {
            (Value::Bool { val: lhs, .. }, Value::Bool { val: rhs, .. }) => Ok(Value::Bool {
                val: *lhs || *rhs,
                span,
            }),
            (Value::CustomValue { val: lhs, span }, rhs) => {
                lhs.operation(*span, Operator::Or, op, rhs)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span()?,
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span()?,
            }),
        }
    }

    pub fn pow(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span()?, rhs.span()?]);

        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if let Some(val) = lhs.checked_pow(*rhs as u32) {
                    Ok(Value::Int { val, span })
                } else {
                    Err(ShellError::OperatorOverflow(
                        "pow operation overflowed".into(),
                        span,
                    ))
                }
            }
            (Value::Int { val: lhs, .. }, Value::Float { val: rhs, .. }) => Ok(Value::Float {
                val: (*lhs as f64).powf(*rhs),
                span,
            }),
            (Value::Float { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Float {
                val: lhs.powf(*rhs as f64),
                span,
            }),
            (Value::Float { val: lhs, .. }, Value::Float { val: rhs, .. }) => Ok(Value::Float {
                val: lhs.powf(*rhs),
                span,
            }),
            (Value::CustomValue { val: lhs, span }, rhs) => {
                lhs.operation(*span, Operator::Pow, op, rhs)
            }

            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span()?,
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span()?,
            }),
        }
    }
}

/// Create a Value::Record from a spanned hashmap
impl From<Spanned<HashMap<String, Value>>> for Value {
    fn from(input: Spanned<HashMap<String, Value>>) -> Self {
        let span = input.span;
        let (cols, vals) = input
            .item
            .into_iter()
            .fold((vec![], vec![]), |mut acc, (k, v)| {
                acc.0.push(k);
                acc.1.push(v);
                acc
            });

        Value::Record { cols, vals, span }
    }
}

/// Create a Value::Record from a spanned indexmap
impl From<Spanned<IndexMap<String, Value>>> for Value {
    fn from(input: Spanned<IndexMap<String, Value>>) -> Self {
        let span = input.span;
        let (cols, vals) = input
            .item
            .into_iter()
            .fold((vec![], vec![]), |mut acc, (k, v)| {
                acc.0.push(k);
                acc.1.push(v);
                acc
            });

        Value::Record { cols, vals, span }
    }
}

/// Format a duration in nanoseconds into a string
pub fn format_duration(duration: i64) -> String {
    let (sign, duration) = if duration >= 0 {
        (1, duration)
    } else {
        (-1, -duration)
    };
    let (micros, nanos): (i64, i64) = (duration / 1000, duration % 1000);
    let (millis, micros): (i64, i64) = (micros / 1000, micros % 1000);
    let (secs, millis): (i64, i64) = (millis / 1000, millis % 1000);
    let (mins, secs): (i64, i64) = (secs / 60, secs % 60);
    let (hours, mins): (i64, i64) = (mins / 60, mins % 60);
    let (days, hours): (i64, i64) = (hours / 24, hours % 24);

    let mut output_prep = vec![];

    if days != 0 {
        output_prep.push(format!("{}day", days));
    }

    if hours != 0 {
        output_prep.push(format!("{}hr", hours));
    }

    if mins != 0 {
        output_prep.push(format!("{}min", mins));
    }
    // output 0sec for zero duration
    if duration == 0 || secs != 0 {
        output_prep.push(format!("{}sec", secs));
    }

    if millis != 0 {
        output_prep.push(format!("{}ms", millis));
    }

    if micros != 0 {
        output_prep.push(format!("{}us", micros));
    }

    if nanos != 0 {
        output_prep.push(format!("{}ns", nanos));
    }

    format!(
        "{}{}",
        if sign == -1 { "-" } else { "" },
        output_prep.join(" ")
    )
}

fn format_filesize(num_bytes: i64, config: &Config) -> String {
    // Allow the user to specify how they want their numbers formatted
    let filesize_format_var = get_config_filesize_format(config);

    let byte = byte_unit::Byte::from_bytes(num_bytes as u128);
    let adj_byte =
        if filesize_format_var.0 == byte_unit::ByteUnit::B && filesize_format_var.1 == "auto" {
            byte.get_appropriate_unit(!config.filesize_metric)
        } else {
            byte.get_adjusted_unit(filesize_format_var.0)
        };

    match adj_byte.get_unit() {
        byte_unit::ByteUnit::B => {
            let locale_string = get_locale().unwrap_or_else(|| String::from("en-US"));
            // Since get_locale() and Locale::from_name() don't always return the same items
            // we need to try and parse it to match. For instance, a valid locale is de_DE
            // however Locale::from_name() wants only de so we split and parse it out.
            let locale_string = locale_string.replace("_", "-"); // en_AU -> en-AU
            let locale = match Locale::from_name(&locale_string) {
                Ok(loc) => loc,
                _ => {
                    let all = num_format::Locale::available_names();
                    let locale_prefix = &locale_string.split('-').collect::<Vec<&str>>();
                    if all.contains(&locale_prefix[0]) {
                        // eprintln!("Found alternate: {}", &locale_prefix[0]);
                        Locale::from_name(locale_prefix[0]).unwrap_or(Locale::en)
                    } else {
                        // eprintln!("Unable to find matching locale. Defaulting to en-US");
                        Locale::en
                    }
                }
            };
            let locale_byte = adj_byte.get_value() as u64;
            let locale_byte_string = locale_byte.to_formatted_string(&locale);

            if filesize_format_var.1 == "auto" {
                format!("{} B", locale_byte_string)
            } else {
                locale_byte_string
            }
        }
        _ => adj_byte.format(1),
    }
}

fn get_config_filesize_format(config: &Config) -> (ByteUnit, &str) {
    // We need to take into account config.filesize_metric so, if someone asks for KB
    // filesize_metric is true, return KiB
    let filesize_format = match config.filesize_format.as_str() {
        "b" => (byte_unit::ByteUnit::B, ""),
        "kb" => {
            if config.filesize_metric {
                (byte_unit::ByteUnit::KiB, "")
            } else {
                (byte_unit::ByteUnit::KB, "")
            }
        }
        "kib" => (byte_unit::ByteUnit::KiB, ""),
        "mb" => {
            if config.filesize_metric {
                (byte_unit::ByteUnit::MiB, "")
            } else {
                (byte_unit::ByteUnit::MB, "")
            }
        }
        "mib" => (byte_unit::ByteUnit::MiB, ""),
        "gb" => {
            if config.filesize_metric {
                (byte_unit::ByteUnit::GiB, "")
            } else {
                (byte_unit::ByteUnit::GB, "")
            }
        }
        "gib" => (byte_unit::ByteUnit::GiB, ""),
        "tb" => {
            if config.filesize_metric {
                (byte_unit::ByteUnit::TiB, "")
            } else {
                (byte_unit::ByteUnit::TB, "")
            }
        }
        "tib" => (byte_unit::ByteUnit::TiB, ""),
        "pb" => {
            if config.filesize_metric {
                (byte_unit::ByteUnit::PiB, "")
            } else {
                (byte_unit::ByteUnit::PB, "")
            }
        }
        "pib" => (byte_unit::ByteUnit::PiB, ""),
        "eb" => {
            if config.filesize_metric {
                (byte_unit::ByteUnit::EiB, "")
            } else {
                (byte_unit::ByteUnit::EB, "")
            }
        }
        "eib" => (byte_unit::ByteUnit::EiB, ""),
        "zb" => {
            if config.filesize_metric {
                (byte_unit::ByteUnit::ZiB, "")
            } else {
                (byte_unit::ByteUnit::ZB, "")
            }
        }
        "zib" => (byte_unit::ByteUnit::ZiB, ""),
        _ => (byte_unit::ByteUnit::B, "auto"),
    };

    filesize_format
}
