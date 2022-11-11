mod custom_value;
mod from;
mod from_value;
mod range;
mod stream;
mod unit;

use crate::ast::{Bits, Boolean, CellPath, Comparison, PathMember};
use crate::ast::{Math, Operator};
use crate::ShellError;
use crate::{did_you_mean, BlockId, Config, Span, Spanned, Type, VarId};
use byte_unit::ByteUnit;
use chrono::{DateTime, Duration, FixedOffset};
use chrono_humanize::HumanTime;
pub use custom_value::CustomValue;
use fancy_regex::Regex;
pub use from_value::FromValue;
use indexmap::map::IndexMap;
use nu_utils::get_system_locale;
use num_format::ToFormattedString;
pub use range::*;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::{Display, Formatter, Result as FmtResult},
    iter,
    path::PathBuf,
    {cmp::Ordering, fmt::Debug},
};
pub use stream::*;
pub use unit::*;

/// Core structured values that pass through the pipeline in Nushell.
// NOTE: Please do not reorder these enum cases without thinking through the
// impact on the PartialOrd implementation and the global sort order
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
    Float {
        val: f64,
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
        span: Span,
    },
    Closure {
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
            Value::Block { val, span } => Value::Block {
                val: *val,
                span: *span,
            },
            Value::Closure {
                val,
                captures,
                span,
            } => Value::Closure {
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
    /// Converts into string values that can be changed into string natively
    pub fn as_string(&self) -> Result<String, ShellError> {
        match self {
            Value::Int { val, .. } => Ok(val.to_string()),
            Value::Float { val, .. } => Ok(val.to_string()),
            Value::String { val, .. } => Ok(val.to_string()),
            Value::Binary { val, .. } => Ok(match std::str::from_utf8(val) {
                Ok(s) => s.to_string(),
                Err(_) => {
                    // println!("{:?}", e);
                    // println!("bytes: {}", pretty_hex::pretty_hex(&val));
                    // panic!("let's see it");
                    return Err(ShellError::CantConvert(
                        "string".into(),
                        "binary".into(),
                        self.span()?,
                        None,
                    ));
                }
            }),
            x => Err(ShellError::CantConvert(
                "string".into(),
                x.get_type().to_string(),
                self.span()?,
                None,
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
                        "string".into(),
                        "binary".into(),
                        self.span()?,
                        None,
                    ))
                }
            }),
            x => Err(ShellError::CantConvert(
                "string".into(),
                x.get_type().to_string(),
                self.span()?,
                None,
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
                None,
            )),
        }
    }

    pub fn as_block(&self) -> Result<BlockId, ShellError> {
        match self {
            Value::Block { val, .. } => Ok(*val),
            Value::Closure { val, .. } => Ok(*val),
            x => Err(ShellError::CantConvert(
                "block".into(),
                x.get_type().to_string(),
                self.span()?,
                None,
            )),
        }
    }

    pub fn as_binary(&self) -> Result<&[u8], ShellError> {
        match self {
            Value::Binary { val, .. } => Ok(val),
            Value::String { val, .. } => Ok(val.as_bytes()),
            x => Err(ShellError::CantConvert(
                "binary".into(),
                x.get_type().to_string(),
                self.span()?,
                None,
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
                None,
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
                None,
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
                None,
            )),
        }
    }

    pub fn as_float(&self) -> Result<f64, ShellError> {
        match self {
            Value::Float { val, .. } => Ok(*val),
            Value::Int { val, .. } => Ok(*val as f64),
            x => Err(ShellError::CantConvert(
                "float".into(),
                x.get_type().to_string(),
                self.span()?,
                None,
            )),
        }
    }

    pub fn as_integer(&self) -> Result<i64, ShellError> {
        match self {
            Value::Int { val, .. } => Ok(*val),
            x => Err(ShellError::CantConvert(
                "integer".into(),
                x.get_type().to_string(),
                self.span()?,
                None,
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
            Value::Closure { span, .. } => Ok(*span),
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
            Value::Closure { span, .. } => *span = new_span,
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
            Value::List { vals, .. } => {
                let mut ty = None;
                for val in vals {
                    let val_ty = val.get_type();
                    match &ty {
                        Some(x) => {
                            if &val_ty != x {
                                if x.is_numeric() && val_ty.is_numeric() {
                                    ty = Some(Type::Number)
                                } else {
                                    ty = Some(Type::Any)
                                }
                            }
                        }
                        None => ty = Some(val_ty),
                    }
                }

                match ty {
                    Some(Type::Record(columns)) => Type::Table(columns),
                    Some(ty) => Type::List(Box::new(ty)),
                    None => Type::List(Box::new(ty.unwrap_or(Type::Any))),
                }
            }
            Value::Nothing { .. } => Type::Nothing,
            Value::Block { .. } => Type::Block,
            Value::Closure { .. } => Type::Closure,
            Value::Error { .. } => Type::Error,
            Value::Binary { .. } => Type::Binary,
            Value::CellPath { .. } => Type::CellPath,
            Value::CustomValue { val, .. } => Type::Custom(val.typetag_name().into()),
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
            Value::Filesize { val, .. } => format_filesize_from_conf(*val, config),
            Value::Duration { val, .. } => format_duration(*val),
            Value::Date { val, .. } => format!("{} ({})", val.to_rfc2822(), HumanTime::from(*val)),
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
            Value::Closure { val, .. } => format!("<Closure {}>", val),
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
            Value::Filesize { val, .. } => format_filesize_from_conf(*val, config),
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
            Value::Closure { val, .. } => format!("<Closure {}>", val),
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
            Value::Filesize { val, .. } => format_filesize_from_conf(*val, config),
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
            Value::Closure { val, .. } => format!("<Closure {}>", val),
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
            Value::List { vals, .. } => vals.is_empty(),
            Value::Record { cols, .. } => cols.is_empty(),
            Value::Binary { val, .. } => val.is_empty(),
            Value::Nothing { .. } => true,
            _ => false,
        }
    }

    pub fn is_nothing(&self) -> bool {
        matches!(self, Value::Nothing { .. })
    }

    /// Create a new `Nothing` value
    pub fn nothing(span: Span) -> Value {
        Value::Nothing { span }
    }

    /// Follow a given cell path into the value: for example accessing select elements in a stream or list
    pub fn follow_cell_path(
        self,
        cell_path: &[PathMember],
        insensitive: bool,
    ) -> Result<Value, ShellError> {
        self.follow_cell_path_helper(cell_path, insensitive, true)
    }

    pub fn follow_cell_path_not_from_user_input(
        self,
        cell_path: &[PathMember],
        insensitive: bool,
    ) -> Result<Value, ShellError> {
        self.follow_cell_path_helper(cell_path, insensitive, false)
    }

    fn follow_cell_path_helper(
        self,
        cell_path: &[PathMember],
        insensitive: bool,
        from_user_input: bool,
    ) -> Result<Value, ShellError> {
        let mut current = self;
        for member in cell_path {
            // FIXME: this uses a few extra clones for simplicity, but there may be a way
            // to traverse the path without them
            match member {
                PathMember::Int {
                    val: count,
                    span: origin_span,
                } => {
                    // Treat a numeric path member as `select <val>`
                    match &mut current {
                        Value::List { vals: val, .. } => {
                            if let Some(item) = val.get(*count) {
                                current = item.clone();
                            } else if val.is_empty() {
                                return Err(ShellError::AccessEmptyContent(*origin_span))
                            } else {
                                return Err(ShellError::AccessBeyondEnd(val.len() - 1, *origin_span));
                            }
                        }
                        Value::Binary { val, .. } => {
                            if let Some(item) = val.get(*count) {
                                current = Value::Int {
                                    val: *item as i64,
                                    span: *origin_span,
                                };
                            } else if val.is_empty() {
                                return Err(ShellError::AccessEmptyContent(*origin_span))
                            } else {
                                return Err(ShellError::AccessBeyondEnd(val.len() - 1, *origin_span));
                            }
                        }
                        Value::Range { val, .. } => {
                            if let Some(item) = val.clone().into_range_iter(None)?.nth(*count) {
                                current = item.clone();
                            } else {
                                return Err(ShellError::AccessBeyondEndOfStream(*origin_span));
                            }
                        }
                        Value::CustomValue { val, .. } => {
                            current = val.follow_path_int(*count, *origin_span)?;
                        }
                        x => {
                            return Err(ShellError::TypeMismatchGenericMessage {
                                err_message: format!("Can't access {} values with a row index. Try specifying a column name instead", x.get_type().to_shape()),
                                span: *origin_span, })
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

                        // Make reverse iterate to avoid duplicate column leads to first value, actuall last value is expected.
                        if let Some(found) = cols.iter().zip(vals.iter()).rev().find(|x| {
                            if insensitive {
                                x.0.to_lowercase() == column_name.to_lowercase()
                            } else {
                                x.0 == column_name
                            }
                        }) {
                            current = found.1.clone();
                        } else {
                            if from_user_input {
                                if let Some(suggestion) = did_you_mean(&cols, column_name) {
                                    return Err(ShellError::DidYouMean(suggestion, *origin_span));
                                }
                            }
                            return Err(ShellError::CantFindColumn(*origin_span, span));
                        }
                    }
                    Value::List { vals, span } => {
                        let mut output = vec![];
                        let mut hasvalue = false;
                        let mut temp: Result<Value, ShellError> = Err(ShellError::NotFound(*span));
                        for val in vals {
                            temp = val.clone().follow_cell_path(
                                &[PathMember::String {
                                    val: column_name.clone(),
                                    span: *origin_span,
                                }],
                                insensitive,
                            );
                            if let Ok(result) = temp.clone() {
                                hasvalue = true;
                                output.push(result);
                            } else {
                                output.push(Value::Nothing { span: *span });
                            }
                        }
                        if hasvalue {
                            current = Value::List {
                                vals: output,
                                span: *span,
                            };
                        } else {
                            return temp;
                        }
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

    /// Follow a given cell path into the value: for example accessing select elements in a stream or list
    pub fn upsert_cell_path(
        &mut self,
        cell_path: &[PathMember],
        callback: Box<dyn FnOnce(&Value) -> Value>,
    ) -> Result<(), ShellError> {
        let orig = self.clone();

        let new_val = callback(&orig.follow_cell_path(cell_path, false)?);

        match new_val {
            Value::Error { error } => Err(error),
            new_val => self.upsert_data_at_cell_path(cell_path, new_val),
        }
    }

    pub fn upsert_data_at_cell_path(
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
                                            col.1.upsert_data_at_cell_path(
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
                                            new_col.upsert_data_at_cell_path(
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
                                    .upsert_data_at_cell_path(&cell_path[1..], new_val.clone())?
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
                                new_col.upsert_data_at_cell_path(&cell_path[1..], new_val)?;
                                vals.push(new_col);
                            }
                        }
                    }
                    v => return Err(ShellError::CantFindColumn(*span, v.span()?)),
                },
                PathMember::Int { val: row_num, span } => match self {
                    Value::List { vals, .. } => {
                        if let Some(v) = vals.get_mut(*row_num) {
                            v.upsert_data_at_cell_path(&cell_path[1..], new_val)?
                        } else if vals.is_empty() {
                            return Err(ShellError::AccessEmptyContent(*span));
                        } else {
                            return Err(ShellError::AccessBeyondEnd(vals.len() - 1, *span));
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

    /// Follow a given cell path into the value: for example accessing select elements in a stream or list
    pub fn update_cell_path(
        &mut self,
        cell_path: &[PathMember],
        callback: Box<dyn FnOnce(&Value) -> Value>,
    ) -> Result<(), ShellError> {
        let orig = self.clone();

        let new_val = callback(&orig.follow_cell_path(cell_path, false)?);

        match new_val {
            Value::Error { error } => Err(error),
            new_val => self.update_data_at_cell_path(cell_path, new_val),
        }
    }

    pub fn update_data_at_cell_path(
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
                                Value::Record {
                                    cols,
                                    vals,
                                    span: v_span,
                                } => {
                                    let mut found = false;
                                    for col in cols.iter().zip(vals.iter_mut()) {
                                        if col.0 == col_name {
                                            found = true;
                                            col.1.update_data_at_cell_path(
                                                &cell_path[1..],
                                                new_val.clone(),
                                            )?
                                        }
                                    }
                                    if !found {
                                        return Err(ShellError::CantFindColumn(*span, *v_span));
                                    }
                                }
                                v => return Err(ShellError::CantFindColumn(*span, v.span()?)),
                            }
                        }
                    }
                    Value::Record {
                        cols,
                        vals,
                        span: v_span,
                    } => {
                        let mut found = false;

                        for col in cols.iter().zip(vals.iter_mut()) {
                            if col.0 == col_name {
                                found = true;

                                col.1
                                    .update_data_at_cell_path(&cell_path[1..], new_val.clone())?
                            }
                        }
                        if !found {
                            return Err(ShellError::CantFindColumn(*span, *v_span));
                        }
                    }
                    v => return Err(ShellError::CantFindColumn(*span, v.span()?)),
                },
                PathMember::Int { val: row_num, span } => match self {
                    Value::List { vals, .. } => {
                        if let Some(v) = vals.get_mut(*row_num) {
                            v.update_data_at_cell_path(&cell_path[1..], new_val)?
                        } else if vals.is_empty() {
                            return Err(ShellError::AccessEmptyContent(*span));
                        } else {
                            return Err(ShellError::AccessBeyondEnd(vals.len() - 1, *span));
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

    pub fn remove_data_at_cell_path(&mut self, cell_path: &[PathMember]) -> Result<(), ShellError> {
        match cell_path.len() {
            0 => Ok(()),
            1 => {
                let path_member = cell_path.first().expect("there is a first");
                match path_member {
                    PathMember::String {
                        val: col_name,
                        span,
                    } => match self {
                        Value::List { vals, .. } => {
                            for val in vals.iter_mut() {
                                match val {
                                    Value::Record {
                                        cols,
                                        vals,
                                        span: v_span,
                                    } => {
                                        let mut found = false;
                                        for (i, col) in cols.clone().iter().enumerate() {
                                            if col == col_name {
                                                cols.remove(i);
                                                vals.remove(i);
                                                found = true;
                                            }
                                        }
                                        if !found {
                                            return Err(ShellError::CantFindColumn(*span, *v_span));
                                        }
                                    }
                                    v => return Err(ShellError::CantFindColumn(*span, v.span()?)),
                                }
                            }
                            Ok(())
                        }
                        Value::Record {
                            cols,
                            vals,
                            span: v_span,
                        } => {
                            let mut found = false;
                            for (i, col) in cols.clone().iter().enumerate() {
                                if col == col_name {
                                    cols.remove(i);
                                    vals.remove(i);
                                    found = true;
                                }
                            }
                            if !found {
                                return Err(ShellError::CantFindColumn(*span, *v_span));
                            }
                            Ok(())
                        }
                        v => Err(ShellError::CantFindColumn(*span, v.span()?)),
                    },
                    PathMember::Int { val: row_num, span } => match self {
                        Value::List { vals, .. } => {
                            if vals.get_mut(*row_num).is_some() {
                                vals.remove(*row_num);
                                Ok(())
                            } else if vals.is_empty() {
                                Err(ShellError::AccessEmptyContent(*span))
                            } else {
                                Err(ShellError::AccessBeyondEnd(vals.len() - 1, *span))
                            }
                        }
                        v => Err(ShellError::NotAList(*span, v.span()?)),
                    },
                }
            }
            _ => {
                let path_member = cell_path.first().expect("there is a first");
                match path_member {
                    PathMember::String {
                        val: col_name,
                        span,
                    } => match self {
                        Value::List { vals, .. } => {
                            for val in vals.iter_mut() {
                                match val {
                                    Value::Record {
                                        cols,
                                        vals,
                                        span: v_span,
                                    } => {
                                        let mut found = false;
                                        for col in cols.iter().zip(vals.iter_mut()) {
                                            if col.0 == col_name {
                                                found = true;
                                                col.1.remove_data_at_cell_path(&cell_path[1..])?
                                            }
                                        }
                                        if !found {
                                            return Err(ShellError::CantFindColumn(*span, *v_span));
                                        }
                                    }
                                    v => return Err(ShellError::CantFindColumn(*span, v.span()?)),
                                }
                            }
                            Ok(())
                        }
                        Value::Record {
                            cols,
                            vals,
                            span: v_span,
                        } => {
                            let mut found = false;

                            for col in cols.iter().zip(vals.iter_mut()) {
                                if col.0 == col_name {
                                    found = true;

                                    col.1.remove_data_at_cell_path(&cell_path[1..])?
                                }
                            }
                            if !found {
                                return Err(ShellError::CantFindColumn(*span, *v_span));
                            }
                            Ok(())
                        }
                        v => Err(ShellError::CantFindColumn(*span, v.span()?)),
                    },
                    PathMember::Int { val: row_num, span } => match self {
                        Value::List { vals, .. } => {
                            if let Some(v) = vals.get_mut(*row_num) {
                                v.remove_data_at_cell_path(&cell_path[1..])
                            } else if vals.is_empty() {
                                Err(ShellError::AccessEmptyContent(*span))
                            } else {
                                Err(ShellError::AccessBeyondEnd(vals.len() - 1, *span))
                            }
                        }
                        v => Err(ShellError::NotAList(*span, v.span()?)),
                    },
                }
            }
        }
    }

    pub fn insert_data_at_cell_path(
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
                                Value::Record {
                                    cols,
                                    vals,
                                    span: v_span,
                                } => {
                                    for col in cols.iter().zip(vals.iter_mut()) {
                                        if col.0 == col_name {
                                            if cell_path.len() == 1 {
                                                return Err(ShellError::ColumnAlreadyExists(
                                                    *span, *v_span,
                                                ));
                                            } else {
                                                return col.1.insert_data_at_cell_path(
                                                    &cell_path[1..],
                                                    new_val,
                                                );
                                            }
                                        }
                                    }

                                    cols.push(col_name.clone());
                                    vals.push(new_val.clone());
                                }
                                _ => {
                                    return Err(ShellError::UnsupportedInput(
                                        "table or record".into(),
                                        *span,
                                    ))
                                }
                            }
                        }
                    }
                    Value::Record {
                        cols,
                        vals,
                        span: v_span,
                    } => {
                        for col in cols.iter().zip(vals.iter_mut()) {
                            if col.0 == col_name {
                                if cell_path.len() == 1 {
                                    return Err(ShellError::ColumnAlreadyExists(*span, *v_span));
                                } else {
                                    return col
                                        .1
                                        .insert_data_at_cell_path(&cell_path[1..], new_val);
                                }
                            }
                        }

                        cols.push(col_name.clone());
                        vals.push(new_val);
                    }
                    _ => {
                        return Err(ShellError::UnsupportedInput(
                            "table or record".into(),
                            *span,
                        ))
                    }
                },
                PathMember::Int { val: row_num, span } => match self {
                    Value::List { vals, .. } => {
                        if let Some(v) = vals.get_mut(*row_num) {
                            v.insert_data_at_cell_path(&cell_path[1..], new_val)?
                        } else if vals.is_empty() {
                            return Err(ShellError::AccessEmptyContent(*span));
                        } else {
                            return Err(ShellError::AccessBeyondEnd(vals.len() - 1, *span));
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

    pub fn is_false(&self) -> bool {
        matches!(self, Value::Bool { val: false, .. })
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

    pub fn binary(val: impl Into<Vec<u8>>, span: Span) -> Value {
        Value::Binary {
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
            (Value::Bool { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Int { .. } => Some(Ordering::Less),
                Value::Float { .. } => Some(Ordering::Less),
                Value::Filesize { .. } => Some(Ordering::Less),
                Value::Duration { .. } => Some(Ordering::Less),
                Value::Date { .. } => Some(Ordering::Less),
                Value::Range { .. } => Some(Ordering::Less),
                Value::String { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Block { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::Int { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Float { val: rhs, .. } => compare_floats(*lhs as f64, *rhs),
                Value::Filesize { .. } => Some(Ordering::Less),
                Value::Duration { .. } => Some(Ordering::Less),
                Value::Date { .. } => Some(Ordering::Less),
                Value::Range { .. } => Some(Ordering::Less),
                Value::String { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Block { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::Float { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { val: rhs, .. } => compare_floats(*lhs, *rhs as f64),
                Value::Float { val: rhs, .. } => compare_floats(*lhs, *rhs),
                Value::Filesize { .. } => Some(Ordering::Less),
                Value::Duration { .. } => Some(Ordering::Less),
                Value::Date { .. } => Some(Ordering::Less),
                Value::Range { .. } => Some(Ordering::Less),
                Value::String { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Block { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::Filesize { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Duration { .. } => Some(Ordering::Less),
                Value::Date { .. } => Some(Ordering::Less),
                Value::Range { .. } => Some(Ordering::Less),
                Value::String { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Block { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::Duration { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Date { .. } => Some(Ordering::Less),
                Value::Range { .. } => Some(Ordering::Less),
                Value::String { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Block { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::Date { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Range { .. } => Some(Ordering::Less),
                Value::String { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Block { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::Range { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { .. } => Some(Ordering::Greater),
                Value::Range { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::String { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Block { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::String { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { .. } => Some(Ordering::Greater),
                Value::Range { .. } => Some(Ordering::Greater),
                Value::String { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Record { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Block { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (
                Value::Record {
                    cols: lhs_cols,
                    vals: lhs_vals,
                    ..
                },
                rhs,
            ) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { .. } => Some(Ordering::Greater),
                Value::Range { .. } => Some(Ordering::Greater),
                Value::String { .. } => Some(Ordering::Greater),
                Value::Record {
                    cols: rhs_cols,
                    vals: rhs_vals,
                    ..
                } => {
                    // reorder cols and vals to make more logically compare.
                    // more genral, if two record have same col and values,
                    // the order of cols shouldn't affect the equal property.
                    let (lhs_cols_ordered, lhs_vals_ordered) =
                        reorder_record_inner(lhs_cols, lhs_vals);
                    let (rhs_cols_ordered, rhs_vals_ordered) =
                        reorder_record_inner(rhs_cols, rhs_vals);

                    let result = lhs_cols_ordered.partial_cmp(&rhs_cols_ordered);
                    if result == Some(Ordering::Equal) {
                        lhs_vals_ordered.partial_cmp(&rhs_vals_ordered)
                    } else {
                        result
                    }
                }
                Value::List { .. } => Some(Ordering::Less),
                Value::Block { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::List { vals: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { .. } => Some(Ordering::Greater),
                Value::Range { .. } => Some(Ordering::Greater),
                Value::String { .. } => Some(Ordering::Greater),
                Value::Record { .. } => Some(Ordering::Greater),
                Value::List { vals: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Block { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::Block { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { .. } => Some(Ordering::Greater),
                Value::Range { .. } => Some(Ordering::Greater),
                Value::String { .. } => Some(Ordering::Greater),
                Value::Record { .. } => Some(Ordering::Greater),
                Value::List { .. } => Some(Ordering::Greater),
                Value::Block { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::Closure { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { .. } => Some(Ordering::Greater),
                Value::Range { .. } => Some(Ordering::Greater),
                Value::String { .. } => Some(Ordering::Greater),
                Value::Record { .. } => Some(Ordering::Greater),
                Value::List { .. } => Some(Ordering::Greater),
                Value::Block { .. } => Some(Ordering::Greater),
                Value::Closure { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::Nothing { .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { .. } => Some(Ordering::Greater),
                Value::Range { .. } => Some(Ordering::Greater),
                Value::String { .. } => Some(Ordering::Greater),
                Value::Record { .. } => Some(Ordering::Greater),
                Value::List { .. } => Some(Ordering::Greater),
                Value::Block { .. } => Some(Ordering::Greater),
                Value::Closure { .. } => Some(Ordering::Greater),
                Value::Nothing { .. } => Some(Ordering::Equal),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::Error { .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { .. } => Some(Ordering::Greater),
                Value::Range { .. } => Some(Ordering::Greater),
                Value::String { .. } => Some(Ordering::Greater),
                Value::Record { .. } => Some(Ordering::Greater),
                Value::List { .. } => Some(Ordering::Greater),
                Value::Block { .. } => Some(Ordering::Greater),
                Value::Closure { .. } => Some(Ordering::Greater),
                Value::Nothing { .. } => Some(Ordering::Greater),
                Value::Error { .. } => Some(Ordering::Equal),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::Binary { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { .. } => Some(Ordering::Greater),
                Value::Range { .. } => Some(Ordering::Greater),
                Value::String { .. } => Some(Ordering::Greater),
                Value::Record { .. } => Some(Ordering::Greater),
                Value::List { .. } => Some(Ordering::Greater),
                Value::Block { .. } => Some(Ordering::Greater),
                Value::Closure { .. } => Some(Ordering::Greater),
                Value::Nothing { .. } => Some(Ordering::Greater),
                Value::Error { .. } => Some(Ordering::Greater),
                Value::Binary { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::CellPath { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { .. } => Some(Ordering::Greater),
                Value::Range { .. } => Some(Ordering::Greater),
                Value::String { .. } => Some(Ordering::Greater),
                Value::Record { .. } => Some(Ordering::Greater),
                Value::List { .. } => Some(Ordering::Greater),
                Value::Block { .. } => Some(Ordering::Greater),
                Value::Closure { .. } => Some(Ordering::Greater),
                Value::Nothing { .. } => Some(Ordering::Greater),
                Value::Error { .. } => Some(Ordering::Greater),
                Value::Binary { .. } => Some(Ordering::Greater),
                Value::CellPath { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::CustomValue { val: lhs, .. }, rhs) => lhs.partial_cmp(rhs),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        self.partial_cmp(other).map_or(false, Ordering::is_eq)
    }
}

impl Value {
    pub fn add(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
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
                lhs.operation(*span, Operator::Math(Math::Plus), op, rhs)
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

    pub fn append(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::List { vals: lhs, .. }, Value::List { vals: rhs, .. }) => {
                let mut lhs = lhs.clone();
                let mut rhs = rhs.clone();
                lhs.append(&mut rhs);
                Ok(Value::List { vals: lhs, span })
            }
            (Value::List { vals: lhs, .. }, val) => {
                let mut lhs = lhs.clone();
                lhs.push(val.clone());
                Ok(Value::List { vals: lhs, span })
            }
            (val, Value::List { vals: rhs, .. }) => {
                let mut rhs = rhs.clone();
                rhs.insert(0, val.clone());
                Ok(Value::List { vals: rhs, span })
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

    pub fn sub(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
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
            (Value::Date { val: lhs, .. }, Value::Date { val: rhs, .. }) => {
                let result = lhs.signed_duration_since(*rhs);

                match result.num_nanoseconds() {
                    Some(v) => Ok(Value::Duration { val: v, span }),
                    None => Err(ShellError::OperatorOverflow(
                        "subtraction operation overflowed".into(),
                        span,
                    )),
                }
            }
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
                lhs.operation(*span, Operator::Math(Math::Minus), op, rhs)
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

    pub fn mul(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
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
            (Value::Int { val: lhs, .. }, Value::Filesize { val: rhs, .. }) => {
                Ok(Value::Filesize {
                    val: *lhs * *rhs,
                    span,
                })
            }
            (Value::Filesize { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                Ok(Value::Filesize {
                    val: *lhs * *rhs,
                    span,
                })
            }
            (Value::Float { val: lhs, .. }, Value::Filesize { val: rhs, .. }) => {
                Ok(Value::Filesize {
                    val: (*lhs * *rhs as f64) as i64,
                    span,
                })
            }
            (Value::Filesize { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                Ok(Value::Filesize {
                    val: (*lhs as f64 * *rhs) as i64,
                    span,
                })
            }
            (Value::Int { val: lhs, .. }, Value::Duration { val: rhs, .. }) => {
                Ok(Value::Duration {
                    val: *lhs * *rhs,
                    span,
                })
            }
            (Value::Duration { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                Ok(Value::Duration {
                    val: *lhs * *rhs,
                    span,
                })
            }
            (Value::Duration { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                Ok(Value::Duration {
                    val: (*lhs as f64 * *rhs) as i64,
                    span,
                })
            }
            (Value::Float { val: lhs, .. }, Value::Duration { val: rhs, .. }) => {
                Ok(Value::Duration {
                    val: (*lhs * *rhs as f64) as i64,
                    span,
                })
            }
            (Value::CustomValue { val: lhs, span }, rhs) => {
                lhs.operation(*span, Operator::Math(Math::Multiply), op, rhs)
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

    pub fn div(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
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
            (Value::Filesize { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if *rhs != 0 {
                    Ok(Value::Filesize {
                        val: ((*lhs as f64) / (*rhs as f64)) as i64,
                        span,
                    })
                } else {
                    Err(ShellError::DivisionByZero(op))
                }
            }
            (Value::Filesize { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                if *rhs != 0.0 {
                    Ok(Value::Filesize {
                        val: (*lhs as f64 / rhs) as i64,
                        span,
                    })
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
            (Value::Duration { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if *rhs != 0 {
                    Ok(Value::Duration {
                        val: ((*lhs as f64) / (*rhs as f64)) as i64,
                        span,
                    })
                } else {
                    Err(ShellError::DivisionByZero(op))
                }
            }
            (Value::Duration { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                if *rhs != 0.0 {
                    Ok(Value::Duration {
                        val: ((*lhs as f64) / rhs) as i64,
                        span,
                    })
                } else {
                    Err(ShellError::DivisionByZero(op))
                }
            }
            (Value::CustomValue { val: lhs, span }, rhs) => {
                lhs.operation(*span, Operator::Math(Math::Divide), op, rhs)
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

    pub fn floor_div(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if *rhs != 0 {
                    Ok(Value::Int {
                        val: (*lhs as f64 / *rhs as f64)
                            .max(std::i64::MIN as f64)
                            .min(std::i64::MAX as f64)
                            .floor() as i64,
                        span,
                    })
                } else {
                    Err(ShellError::DivisionByZero(op))
                }
            }
            (Value::Int { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                if *rhs != 0.0 {
                    Ok(Value::Int {
                        val: (*lhs as f64 / *rhs)
                            .max(std::i64::MIN as f64)
                            .min(std::i64::MAX as f64)
                            .floor() as i64,
                        span,
                    })
                } else {
                    Err(ShellError::DivisionByZero(op))
                }
            }
            (Value::Float { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if *rhs != 0 {
                    Ok(Value::Int {
                        val: (*lhs / *rhs as f64)
                            .max(std::i64::MIN as f64)
                            .min(std::i64::MAX as f64)
                            .floor() as i64,
                        span,
                    })
                } else {
                    Err(ShellError::DivisionByZero(op))
                }
            }
            (Value::Float { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                if *rhs != 0.0 {
                    Ok(Value::Int {
                        val: (lhs / rhs)
                            .max(std::i64::MIN as f64)
                            .min(std::i64::MAX as f64)
                            .floor() as i64,
                        span,
                    })
                } else {
                    Err(ShellError::DivisionByZero(op))
                }
            }
            (Value::Filesize { val: lhs, .. }, Value::Filesize { val: rhs, .. }) => {
                if *rhs != 0 {
                    Ok(Value::Int {
                        val: (*lhs as f64 / *rhs as f64)
                            .max(std::i64::MIN as f64)
                            .min(std::i64::MAX as f64)
                            .floor() as i64,
                        span,
                    })
                } else {
                    Err(ShellError::DivisionByZero(op))
                }
            }
            (Value::Filesize { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if *rhs != 0 {
                    Ok(Value::Filesize {
                        val: ((*lhs as f64) / (*rhs as f64))
                            .max(std::i64::MIN as f64)
                            .min(std::i64::MAX as f64)
                            .floor() as i64,
                        span,
                    })
                } else {
                    Err(ShellError::DivisionByZero(op))
                }
            }
            (Value::Filesize { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                if *rhs != 0.0 {
                    Ok(Value::Filesize {
                        val: (*lhs as f64 / *rhs)
                            .max(std::i64::MIN as f64)
                            .min(std::i64::MAX as f64)
                            .floor() as i64,
                        span,
                    })
                } else {
                    Err(ShellError::DivisionByZero(op))
                }
            }
            (Value::Duration { val: lhs, .. }, Value::Duration { val: rhs, .. }) => {
                if *rhs != 0 {
                    Ok(Value::Int {
                        val: (*lhs as f64 / *rhs as f64)
                            .max(std::i64::MIN as f64)
                            .min(std::i64::MAX as f64)
                            .floor() as i64,
                        span,
                    })
                } else {
                    Err(ShellError::DivisionByZero(op))
                }
            }
            (Value::Duration { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if *rhs != 0 {
                    Ok(Value::Duration {
                        val: (*lhs as f64 / *rhs as f64)
                            .max(std::i64::MIN as f64)
                            .min(std::i64::MAX as f64)
                            .floor() as i64,
                        span,
                    })
                } else {
                    Err(ShellError::DivisionByZero(op))
                }
            }
            (Value::Duration { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                if *rhs != 0.0 {
                    Ok(Value::Duration {
                        val: (*lhs as f64 / *rhs)
                            .max(std::i64::MIN as f64)
                            .min(std::i64::MAX as f64)
                            .floor() as i64,
                        span,
                    })
                } else {
                    Err(ShellError::DivisionByZero(op))
                }
            }
            (Value::CustomValue { val: lhs, span }, rhs) => {
                lhs.operation(*span, Operator::Math(Math::Divide), op, rhs)
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

    pub fn lt(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        if let (Value::CustomValue { val: lhs, span }, rhs) = (self, rhs) {
            return lhs.operation(*span, Operator::Comparison(Comparison::LessThan), op, rhs);
        }

        if !type_compatible(self.get_type(), rhs.get_type())
            && (self.get_type() != Type::Any)
            && (rhs.get_type() != Type::Any)
        {
            return Err(ShellError::TypeMismatch("compatible type".to_string(), op));
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

    pub fn lte(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        if let (Value::CustomValue { val: lhs, span }, rhs) = (self, rhs) {
            return lhs.operation(
                *span,
                Operator::Comparison(Comparison::LessThanOrEqual),
                op,
                rhs,
            );
        }

        if !type_compatible(self.get_type(), rhs.get_type())
            && (self.get_type() != Type::Any)
            && (rhs.get_type() != Type::Any)
        {
            return Err(ShellError::TypeMismatch("compatible type".to_string(), op));
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

    pub fn gt(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        if let (Value::CustomValue { val: lhs, span }, rhs) = (self, rhs) {
            return lhs.operation(
                *span,
                Operator::Comparison(Comparison::GreaterThan),
                op,
                rhs,
            );
        }

        if !type_compatible(self.get_type(), rhs.get_type())
            && (self.get_type() != Type::Any)
            && (rhs.get_type() != Type::Any)
        {
            return Err(ShellError::TypeMismatch("compatible type".to_string(), op));
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

    pub fn gte(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        if let (Value::CustomValue { val: lhs, span }, rhs) = (self, rhs) {
            return lhs.operation(
                *span,
                Operator::Comparison(Comparison::GreaterThanOrEqual),
                op,
                rhs,
            );
        }

        if !type_compatible(self.get_type(), rhs.get_type())
            && (self.get_type() != Type::Any)
            && (rhs.get_type() != Type::Any)
        {
            return Err(ShellError::TypeMismatch("compatible type".to_string(), op));
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

    pub fn eq(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        if let (Value::CustomValue { val: lhs, span }, rhs) = (self, rhs) {
            return lhs.operation(*span, Operator::Comparison(Comparison::Equal), op, rhs);
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

    pub fn ne(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        if let (Value::CustomValue { val: lhs, span }, rhs) = (self, rhs) {
            return lhs.operation(*span, Operator::Comparison(Comparison::NotEqual), op, rhs);
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

    pub fn r#in(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
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
                lhs.operation(*span, Operator::Comparison(Comparison::In), op, rhs)
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

    pub fn not_in(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
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
                lhs.operation(*span, Operator::Comparison(Comparison::NotIn), op, rhs)
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

    pub fn regex_match(
        &self,
        op: Span,
        rhs: &Value,
        invert: bool,
        span: Span,
    ) -> Result<Value, ShellError> {
        match (self, rhs) {
            (
                Value::String { val: lhs, .. },
                Value::String {
                    val: rhs,
                    span: rhs_span,
                },
            ) => {
                // We are leaving some performance on the table by compiling the regex every time.
                // Small regexes compile in microseconds, and the simplicity of this approach currently
                // outweighs the performance costs. Revisit this if it ever becomes a bottleneck.
                let regex = Regex::new(rhs)
                    .map_err(|e| ShellError::UnsupportedInput(format!("{e}"), *rhs_span))?;
                let is_match = regex.is_match(lhs);
                Ok(Value::Bool {
                    val: if invert {
                        !is_match.unwrap_or(false)
                    } else {
                        is_match.unwrap_or(true)
                    },
                    span,
                })
            }
            (Value::CustomValue { val: lhs, span }, rhs) => lhs.operation(
                *span,
                if invert {
                    Operator::Comparison(Comparison::NotRegexMatch)
                } else {
                    Operator::Comparison(Comparison::RegexMatch)
                },
                op,
                rhs,
            ),
            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span()?,
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span()?,
            }),
        }
    }

    pub fn starts_with(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::String { val: lhs, .. }, Value::String { val: rhs, .. }) => Ok(Value::Bool {
                val: lhs.starts_with(rhs),
                span,
            }),
            (Value::CustomValue { val: lhs, span }, rhs) => {
                lhs.operation(*span, Operator::Comparison(Comparison::StartsWith), op, rhs)
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

    pub fn ends_with(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::String { val: lhs, .. }, Value::String { val: rhs, .. }) => Ok(Value::Bool {
                val: lhs.ends_with(rhs),
                span,
            }),
            (Value::CustomValue { val: lhs, span }, rhs) => {
                lhs.operation(*span, Operator::Comparison(Comparison::EndsWith), op, rhs)
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

    pub fn bit_shl(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Int {
                span,
                val: *lhs << rhs,
            }),
            (Value::CustomValue { val: lhs, span }, rhs) => {
                lhs.operation(*span, Operator::Bits(Bits::ShiftLeft), op, rhs)
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

    pub fn bit_shr(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Int {
                span,
                val: *lhs >> rhs,
            }),
            (Value::CustomValue { val: lhs, span }, rhs) => {
                lhs.operation(*span, Operator::Bits(Bits::ShiftRight), op, rhs)
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

    pub fn bit_or(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Int {
                span,
                val: *lhs | rhs,
            }),
            (Value::CustomValue { val: lhs, span }, rhs) => {
                lhs.operation(*span, Operator::Bits(Bits::BitOr), op, rhs)
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

    pub fn bit_xor(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Int {
                span,
                val: *lhs ^ rhs,
            }),
            (Value::CustomValue { val: lhs, span }, rhs) => {
                lhs.operation(*span, Operator::Bits(Bits::BitXor), op, rhs)
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

    pub fn bit_and(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Int {
                span,
                val: *lhs & rhs,
            }),
            (Value::CustomValue { val: lhs, span }, rhs) => {
                lhs.operation(*span, Operator::Bits(Bits::BitAnd), op, rhs)
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

    pub fn modulo(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
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
                lhs.operation(*span, Operator::Math(Math::Modulo), op, rhs)
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

    pub fn and(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::Bool { val: lhs, .. }, Value::Bool { val: rhs, .. }) => Ok(Value::Bool {
                val: *lhs && *rhs,
                span,
            }),
            (Value::CustomValue { val: lhs, span }, rhs) => {
                lhs.operation(*span, Operator::Boolean(Boolean::And), op, rhs)
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

    pub fn or(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::Bool { val: lhs, .. }, Value::Bool { val: rhs, .. }) => Ok(Value::Bool {
                val: *lhs || *rhs,
                span,
            }),
            (Value::CustomValue { val: lhs, span }, rhs) => {
                lhs.operation(*span, Operator::Boolean(Boolean::Or), op, rhs)
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

    pub fn pow(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
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
                lhs.operation(*span, Operator::Math(Math::Pow), op, rhs)
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

fn reorder_record_inner(cols: &[String], vals: &[Value]) -> (Vec<String>, Vec<Value>) {
    let mut kv_pairs =
        iter::zip(cols.to_owned(), vals.to_owned()).collect::<Vec<(String, Value)>>();
    kv_pairs.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .expect("Columns should support compare")
    });
    let (mut cols, mut vals) = (vec![], vec![]);
    for (col, val) in kv_pairs {
        cols.push(col);
        vals.push(val);
    }
    (cols, vals)
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

fn type_compatible(a: Type, b: Type) -> bool {
    if a == b {
        return true;
    }

    matches!((a, b), (Type::Int, Type::Float) | (Type::Float, Type::Int))
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

/// Is the given year a leap year?
#[allow(clippy::nonminimal_bool)]
pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0) && (year % 100 != 0 || (year % 100 == 0 && year % 400 == 0))
}

#[derive(Clone, Copy)]
enum TimePeriod {
    Nanos(i64),
    Micros(i64),
    Millis(i64),
    Seconds(i64),
    Minutes(i64),
    Hours(i64),
    Days(i64),
    Weeks(i64),
    Months(i64),
    Years(i64),
}

impl TimePeriod {
    fn to_text(self) -> Cow<'static, str> {
        match self {
            Self::Nanos(n) => format!("{}ns", n).into(),
            Self::Micros(n) => format!("{}µs", n).into(),
            Self::Millis(n) => format!("{}ms", n).into(),
            Self::Seconds(n) => format!("{}sec", n).into(),
            Self::Minutes(n) => format!("{}min", n).into(),
            Self::Hours(n) => format!("{}hr", n).into(),
            Self::Days(n) => format!("{}day", n).into(),
            Self::Weeks(n) => format!("{}wk", n).into(),
            Self::Months(n) => format!("{}month", n).into(),
            Self::Years(n) => format!("{}yr", n).into(),
        }
    }
}

impl Display for TimePeriod {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.to_text())
    }
}

pub fn format_duration(duration: i64) -> String {
    // Attribution: most of this is taken from chrono-humanize-rs. Thanks!
    // https://gitlab.com/imp/chrono-humanize-rs/-/blob/master/src/humantime.rs
    const DAYS_IN_YEAR: i64 = 365;
    const DAYS_IN_MONTH: i64 = 30;

    let (sign, duration) = if duration >= 0 {
        (1, duration)
    } else {
        (-1, -duration)
    };

    let dur = Duration::nanoseconds(duration);

    /// Split this a duration into number of whole years and the remainder
    fn split_years(duration: Duration) -> (Option<i64>, Duration) {
        let years = duration.num_days() / DAYS_IN_YEAR;
        let remainder = duration - Duration::days(years * DAYS_IN_YEAR);
        normalize_split(years, remainder)
    }

    /// Split this a duration into number of whole months and the remainder
    fn split_months(duration: Duration) -> (Option<i64>, Duration) {
        let months = duration.num_days() / DAYS_IN_MONTH;
        let remainder = duration - Duration::days(months * DAYS_IN_MONTH);
        normalize_split(months, remainder)
    }

    /// Split this a duration into number of whole weeks and the remainder
    fn split_weeks(duration: Duration) -> (Option<i64>, Duration) {
        let weeks = duration.num_weeks();
        let remainder = duration - Duration::weeks(weeks);
        normalize_split(weeks, remainder)
    }

    /// Split this a duration into number of whole days and the remainder
    fn split_days(duration: Duration) -> (Option<i64>, Duration) {
        let days = duration.num_days();
        let remainder = duration - Duration::days(days);
        normalize_split(days, remainder)
    }

    /// Split this a duration into number of whole hours and the remainder
    fn split_hours(duration: Duration) -> (Option<i64>, Duration) {
        let hours = duration.num_hours();
        let remainder = duration - Duration::hours(hours);
        normalize_split(hours, remainder)
    }

    /// Split this a duration into number of whole minutes and the remainder
    fn split_minutes(duration: Duration) -> (Option<i64>, Duration) {
        let minutes = duration.num_minutes();
        let remainder = duration - Duration::minutes(minutes);
        normalize_split(minutes, remainder)
    }

    /// Split this a duration into number of whole seconds and the remainder
    fn split_seconds(duration: Duration) -> (Option<i64>, Duration) {
        let seconds = duration.num_seconds();
        let remainder = duration - Duration::seconds(seconds);
        normalize_split(seconds, remainder)
    }

    /// Split this a duration into number of whole milliseconds and the remainder
    fn split_milliseconds(duration: Duration) -> (Option<i64>, Duration) {
        let millis = duration.num_milliseconds();
        let remainder = duration - Duration::milliseconds(millis);
        normalize_split(millis, remainder)
    }

    /// Split this a duration into number of whole seconds and the remainder
    fn split_microseconds(duration: Duration) -> (Option<i64>, Duration) {
        let micros = duration.num_microseconds().unwrap_or_default();
        let remainder = duration - Duration::microseconds(micros);
        normalize_split(micros, remainder)
    }

    /// Split this a duration into number of whole seconds and the remainder
    fn split_nanoseconds(duration: Duration) -> (Option<i64>, Duration) {
        let nanos = duration.num_nanoseconds().unwrap_or_default();
        let remainder = duration - Duration::nanoseconds(nanos);
        normalize_split(nanos, remainder)
    }

    fn normalize_split(
        wholes: impl Into<Option<i64>>,
        remainder: Duration,
    ) -> (Option<i64>, Duration) {
        let wholes = wholes.into().map(i64::abs).filter(|x| *x > 0);
        (wholes, remainder)
    }

    let mut periods = vec![];
    let (years, remainder) = split_years(dur);
    if let Some(years) = years {
        periods.push(TimePeriod::Years(years));
    }

    let (months, remainder) = split_months(remainder);
    if let Some(months) = months {
        periods.push(TimePeriod::Months(months));
    }

    let (weeks, remainder) = split_weeks(remainder);
    if let Some(weeks) = weeks {
        periods.push(TimePeriod::Weeks(weeks));
    }

    let (days, remainder) = split_days(remainder);
    if let Some(days) = days {
        periods.push(TimePeriod::Days(days));
    }

    let (hours, remainder) = split_hours(remainder);
    if let Some(hours) = hours {
        periods.push(TimePeriod::Hours(hours));
    }

    let (minutes, remainder) = split_minutes(remainder);
    if let Some(minutes) = minutes {
        periods.push(TimePeriod::Minutes(minutes));
    }

    let (seconds, remainder) = split_seconds(remainder);
    if let Some(seconds) = seconds {
        periods.push(TimePeriod::Seconds(seconds));
    }

    let (millis, remainder) = split_milliseconds(remainder);
    if let Some(millis) = millis {
        periods.push(TimePeriod::Millis(millis));
    }

    let (micros, remainder) = split_microseconds(remainder);
    if let Some(micros) = micros {
        periods.push(TimePeriod::Micros(micros));
    }

    let (nanos, _remainder) = split_nanoseconds(remainder);
    if let Some(nanos) = nanos {
        periods.push(TimePeriod::Nanos(nanos));
    }

    if periods.is_empty() {
        periods.push(TimePeriod::Seconds(0));
    }

    // let last = periods.pop().map(|last| last.to_text().to_string());
    let text = periods
        .into_iter()
        .map(|p| p.to_text().to_string())
        .collect::<Vec<String>>();

    // if let Some(last) = last {
    //     text.push(format!("and {}", last));
    // }

    format!(
        "{}{}",
        if sign == -1 { "-" } else { "" },
        text.join(" ").trim()
    )
}

pub fn format_filesize_from_conf(num_bytes: i64, config: &Config) -> String {
    // We need to take into account config.filesize_metric so, if someone asks for KB
    // filesize_metric is true, return KiB
    format_filesize(
        num_bytes,
        config.filesize_format.as_str(),
        config.filesize_metric,
    )
}

pub fn format_filesize(num_bytes: i64, format_value: &str, filesize_metric: bool) -> String {
    // Allow the user to specify how they want their numbers formatted
    let filesize_format_var = get_filesize_format(format_value, filesize_metric);

    let byte = byte_unit::Byte::from_bytes(num_bytes.unsigned_abs() as u128);
    let adj_byte =
        if filesize_format_var.0 == byte_unit::ByteUnit::B && filesize_format_var.1 == "auto" {
            byte.get_appropriate_unit(!filesize_metric)
        } else {
            byte.get_adjusted_unit(filesize_format_var.0)
        };

    match adj_byte.get_unit() {
        byte_unit::ByteUnit::B => {
            let locale = get_system_locale();
            let locale_byte = adj_byte.get_value() as u64;
            let locale_byte_string = locale_byte.to_formatted_string(&locale);
            let locale_signed_byte_string = if num_bytes.is_negative() {
                format!("-{}", locale_byte_string)
            } else {
                locale_byte_string
            };

            if filesize_format_var.1 == "auto" {
                format!("{} B", locale_signed_byte_string)
            } else {
                locale_signed_byte_string
            }
        }
        _ => {
            if num_bytes.is_negative() {
                format!("-{}", adj_byte.format(1))
            } else {
                adj_byte.format(1)
            }
        }
    }
}

fn get_filesize_format(format_value: &str, filesize_metric: bool) -> (ByteUnit, &str) {
    match format_value {
        "b" => (byte_unit::ByteUnit::B, ""),
        "kb" => {
            if filesize_metric {
                (byte_unit::ByteUnit::KiB, "")
            } else {
                (byte_unit::ByteUnit::KB, "")
            }
        }
        "kib" => (byte_unit::ByteUnit::KiB, ""),
        "mb" => {
            if filesize_metric {
                (byte_unit::ByteUnit::MiB, "")
            } else {
                (byte_unit::ByteUnit::MB, "")
            }
        }
        "mib" => (byte_unit::ByteUnit::MiB, ""),
        "gb" => {
            if filesize_metric {
                (byte_unit::ByteUnit::GiB, "")
            } else {
                (byte_unit::ByteUnit::GB, "")
            }
        }
        "gib" => (byte_unit::ByteUnit::GiB, ""),
        "tb" => {
            if filesize_metric {
                (byte_unit::ByteUnit::TiB, "")
            } else {
                (byte_unit::ByteUnit::TB, "")
            }
        }
        "tib" => (byte_unit::ByteUnit::TiB, ""),
        "pb" => {
            if filesize_metric {
                (byte_unit::ByteUnit::PiB, "")
            } else {
                (byte_unit::ByteUnit::PB, "")
            }
        }
        "pib" => (byte_unit::ByteUnit::PiB, ""),
        "eb" => {
            if filesize_metric {
                (byte_unit::ByteUnit::EiB, "")
            } else {
                (byte_unit::ByteUnit::EB, "")
            }
        }
        "eib" => (byte_unit::ByteUnit::EiB, ""),
        "zb" => {
            if filesize_metric {
                (byte_unit::ByteUnit::ZiB, "")
            } else {
                (byte_unit::ByteUnit::ZB, "")
            }
        }
        "zib" => (byte_unit::ByteUnit::ZiB, ""),
        _ => (byte_unit::ByteUnit::B, "auto"),
    }
}

#[cfg(test)]
mod tests {

    use super::{Span, Value};

    mod is_empty {
        use super::*;

        #[test]
        fn test_string() {
            let value = Value::string("", Span::unknown());
            assert!(value.is_empty());
        }

        #[test]
        fn test_list() {
            let list_with_no_values = Value::List {
                vals: vec![],
                span: Span::unknown(),
            };
            let list_with_one_empty_string = Value::List {
                vals: vec![Value::string("", Span::unknown())],
                span: Span::unknown(),
            };

            assert!(list_with_no_values.is_empty());
            assert!(!list_with_one_empty_string.is_empty());
        }

        #[test]
        fn test_record() {
            let no_columns_nor_cell_values = Value::Record {
                cols: vec![],
                vals: vec![],
                span: Span::unknown(),
            };
            let one_column_and_one_cell_value_with_empty_strings = Value::Record {
                cols: vec![String::from("")],
                vals: vec![Value::string("", Span::unknown())],
                span: Span::unknown(),
            };
            let one_column_with_a_string_and_one_cell_value_with_empty_string = Value::Record {
                cols: vec![String::from("column")],
                vals: vec![Value::string("", Span::unknown())],
                span: Span::unknown(),
            };
            let one_column_with_empty_string_and_one_value_with_a_string = Value::Record {
                cols: vec![String::from("")],
                vals: vec![Value::string("text", Span::unknown())],
                span: Span::unknown(),
            };

            assert!(no_columns_nor_cell_values.is_empty());
            assert!(!one_column_and_one_cell_value_with_empty_strings.is_empty());
            assert!(!one_column_with_a_string_and_one_cell_value_with_empty_string.is_empty());
            assert!(!one_column_with_empty_string_and_one_value_with_a_string.is_empty());
        }
    }

    mod get_type {
        use crate::Type;

        use super::*;

        #[test]
        fn test_list() {
            let list_of_ints = Value::List {
                vals: vec![Value::Int {
                    val: 0,
                    span: Span::unknown(),
                }],
                span: Span::unknown(),
            };
            let list_of_floats = Value::List {
                vals: vec![Value::Float {
                    val: 0.0,
                    span: Span::unknown(),
                }],
                span: Span::unknown(),
            };
            let list_of_ints_and_floats = Value::List {
                vals: vec![
                    Value::Int {
                        val: 0,
                        span: Span::unknown(),
                    },
                    Value::Float {
                        val: 0.0,
                        span: Span::unknown(),
                    },
                ],
                span: Span::unknown(),
            };
            let list_of_ints_and_floats_and_bools = Value::List {
                vals: vec![
                    Value::Int {
                        val: 0,
                        span: Span::unknown(),
                    },
                    Value::Float {
                        val: 0.0,
                        span: Span::unknown(),
                    },
                    Value::Bool {
                        val: false,
                        span: Span::unknown(),
                    },
                ],
                span: Span::unknown(),
            };
            assert_eq!(list_of_ints.get_type(), Type::List(Box::new(Type::Int)));
            assert_eq!(list_of_floats.get_type(), Type::List(Box::new(Type::Float)));
            assert_eq!(
                list_of_ints_and_floats_and_bools.get_type(),
                Type::List(Box::new(Type::Any))
            );
            assert_eq!(
                list_of_ints_and_floats.get_type(),
                Type::List(Box::new(Type::Number))
            );
        }
    }
}
