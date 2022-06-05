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
use regex::Regex;
use serde::{Deserialize, Serialize};
pub use stream::*;
use sys_locale::get_locale;
pub use unit::*;

use std::collections::HashMap;
use std::path::PathBuf;
use std::{cmp::Ordering, fmt::Debug};

use crate::ast::{CellPath, PathMember};
use crate::{did_you_mean, BlockId, Config, Span, Spanned, Type, VarId};

use crate::ast::Operator;
pub use custom_value::CustomValue;
use std::iter;

use crate::ShellError;

/// Core structured values that pass through the pipeline in Nushell.
// NOTE: Please do not reorder these enum cases without thinking through the
// impact on the PartialOrd implementation and the global sort order
#[derive(Debug, Serialize, Deserialize)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    Filesize(i64),
    Duration(i64),
    Date(DateTime<FixedOffset>),
    Range(Box<Range>),
    String(String),
    Record {
        cols: Vec<String>,
        vals: Vec<Value>,
    },
    List(Vec<Value>),
    Block {
        val: BlockId,
        captures: HashMap<VarId, Value>,
    },
    Nothing,
    Error(ShellError),
    Binary(Vec<u8>),
    CellPath(CellPath),
    CustomValue(Box<dyn CustomValue>),
}

impl Clone for Value {
    fn clone(&self) -> Self {
        match self {
            Value::Bool(val) => Value::Bool(*val),
            Value::Int(val) => Value::Int(*val),
            Value::Filesize(val) => Value::Filesize(*val),
            Value::Duration(val) => Value::Duration(*val),
            Value::Date(val) => Value::Date(*val),
            Value::Range(val) => Value::Range(val.clone()),
            Value::Float(val) => Value::Float(*val),
            Value::String(val) => Value::String(val.clone()),
            Value::Record { cols, vals } => Value::Record {
                cols: cols.clone(),
                vals: vals.clone(),
            },
            Value::List(vals) => Value::List(vals.clone()),
            Value::Block { val, captures } => Value::Block {
                val: *val,
                captures: captures.clone(),
            },
            Value::Nothing => Value::Nothing,
            Value::Error(error) => Value::Error(error.clone()),
            Value::Binary(val) => Value::Binary(val.clone()),
            Value::CellPath(val) => Value::CellPath(val.clone()),
            Value::CustomValue(val) => val.clone_value(),
        }
    }
}

impl Value {
    /// Converts into string values that can be changed into string natively
    pub fn as_string(&self, span: Span) -> Result<String, ShellError> {
        match self {
            Value::Int(val) => Ok(val.to_string()),
            Value::Float(val) => Ok(val.to_string()),
            Value::String(val) => Ok(val.to_string()),
            Value::Binary(val) => Ok(match std::str::from_utf8(val) {
                Ok(s) => s.to_string(),
                Err(_) => {
                    // println!("{:?}", e);
                    // println!("bytes: {}", pretty_hex::pretty_hex(&val));
                    // panic!("let's see it");
                    return Err(ShellError::CantConvert(
                        "string".into(),
                        "binary".into(),
                        span,
                        None,
                    ));
                }
            }),
            x => Err(ShellError::CantConvert(
                "string".into(),
                x.get_type().to_string(),
                span,
                None,
            )),
        }
    }

    pub fn as_spanned_string(&self, span: Span) -> Result<Spanned<String>, ShellError> {
        match self {
            Value::String(val) => Ok(Spanned {
                item: val.to_string(),
                span,
            }),
            Value::Binary(val) => Ok(match std::str::from_utf8(val) {
                Ok(s) => Spanned {
                    item: s.to_string(),
                    span,
                },
                Err(_) => {
                    return Err(ShellError::CantConvert(
                        "string".into(),
                        "binary".into(),
                        span,
                        None,
                    ))
                }
            }),
            x => Err(ShellError::CantConvert(
                "string".into(),
                x.get_type().to_string(),
                span,
                None,
            )),
        }
    }

    pub fn as_path(&self, span: Span) -> Result<PathBuf, ShellError> {
        match self {
            Value::String(val) => Ok(PathBuf::from(val)),
            x => Err(ShellError::CantConvert(
                "path".into(),
                x.get_type().to_string(),
                span,
                None,
            )),
        }
    }

    pub fn as_block(&self, span: Span) -> Result<BlockId, ShellError> {
        match self {
            Value::Block { val, .. } => Ok(*val),
            x => Err(ShellError::CantConvert(
                "block".into(),
                x.get_type().to_string(),
                span,
                None,
            )),
        }
    }

    pub fn as_binary(&self, span: Span) -> Result<&[u8], ShellError> {
        match self {
            Value::Binary(val) => Ok(val),
            Value::String(val) => Ok(val.as_bytes()),
            x => Err(ShellError::CantConvert(
                "binary".into(),
                x.get_type().to_string(),
                span,
                None,
            )),
        }
    }

    pub fn as_record(&self, span: Span) -> Result<(&[String], &[Value]), ShellError> {
        match self {
            Value::Record { cols, vals, .. } => Ok((cols, vals)),
            x => Err(ShellError::CantConvert(
                "record".into(),
                x.get_type().to_string(),
                span,
                None,
            )),
        }
    }

    pub fn as_list(&self, span: Span) -> Result<&[Value], ShellError> {
        match self {
            Value::List(vals) => Ok(vals),
            x => Err(ShellError::CantConvert(
                "list".into(),
                x.get_type().to_string(),
                span,
                None,
            )),
        }
    }

    pub fn as_bool(&self, span: Span) -> Result<bool, ShellError> {
        match self {
            Value::Bool(val) => Ok(*val),
            x => Err(ShellError::CantConvert(
                "boolean".into(),
                x.get_type().to_string(),
                span,
                None,
            )),
        }
    }

    pub fn as_float(&self, span: Span) -> Result<f64, ShellError> {
        match self {
            Value::Float(val) => Ok(*val),
            Value::Int(val) => Ok(*val as f64),
            x => Err(ShellError::CantConvert(
                "float".into(),
                x.get_type().to_string(),
                span,
                None,
            )),
        }
    }

    pub fn as_integer(&self, span: Span) -> Result<i64, ShellError> {
        match self {
            Value::Int(val) => Ok(*val),
            x => Err(ShellError::CantConvert(
                "integer".into(),
                x.get_type().to_string(),
                span,
                None,
            )),
        }
    }

    /// Get the type of the current Value
    pub fn get_type(&self) -> Type {
        match self {
            Value::Bool(_) => Type::Bool,
            Value::Int(_) => Type::Int,
            Value::Float(_) => Type::Float,
            Value::Filesize(_) => Type::Filesize,
            Value::Duration(_) => Type::Duration,
            Value::Date(_) => Type::Date,
            Value::Range(_) => Type::Range,
            Value::String(_) => Type::String,
            Value::Record { cols, vals } => Type::Record(
                cols.iter()
                    .zip(vals.iter())
                    .map(|(x, y)| (x.clone(), y.get_type()))
                    .collect(),
            ),
            Value::List(vals) => {
                let mut ty = None;
                for val in vals {
                    let val_ty = val.get_type();
                    match &ty {
                        Some(x) => {
                            if &val_ty != x {
                                ty = Some(Type::Any)
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
            Value::Nothing => Type::Nothing,
            Value::Block { .. } => Type::Block,
            Value::Error(_) => Type::Error,
            Value::Binary(_) => Type::Binary,
            Value::CellPath(_) => Type::CellPath,
            Value::CustomValue(_) => Type::Custom,
        }
    }

    pub fn get_data_by_key(&self, name: &str) -> Option<Value> {
        match self {
            Value::Record { cols, vals } => cols
                .iter()
                .zip(vals.iter())
                .find(|(col, _)| col == &name)
                .map(|(_, val)| val.clone()),
            Value::List(vals) => {
                let mut out = vec![];
                for item in vals {
                    match item {
                        Value::Record { .. } => match item.get_data_by_key(name) {
                            Some(v) => out.push(v),
                            None => out.push(Value::Nothing),
                        },
                        _ => out.push(Value::Nothing),
                    }
                }

                if !out.is_empty() {
                    Some(Value::List(out))
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
            Value::Bool(val) => val.to_string(),
            Value::Int(val) => val.to_string(),
            Value::Float(val) => val.to_string(),
            Value::Filesize(val) => format_filesize_from_conf(*val, config),
            Value::Duration(val) => format_duration(*val),
            Value::Date(val) => format!("{} ({})", val.to_rfc2822(), HumanTime::from(*val)),
            Value::Range(val) => {
                format!(
                    "{}..{}",
                    val.from.into_string(", ", config),
                    val.to.into_string(", ", config)
                )
            }
            Value::String(val) => val.clone(),
            Value::List(val) => format!(
                "[{}]",
                val.iter()
                    .map(|x| x.into_string(", ", config))
                    .collect::<Vec<_>>()
                    .join(separator)
            ),
            Value::Record { cols, vals } => format!(
                "{{{}}}",
                cols.iter()
                    .zip(vals.iter())
                    .map(|(x, y)| format!("{}: {}", x, y.into_string(", ", config)))
                    .collect::<Vec<_>>()
                    .join(separator)
            ),
            Value::Block { val, .. } => format!("<Block {}>", val),
            Value::Nothing => String::new(),
            Value::Error(error) => format!("{:?}", error),
            Value::Binary(val) => format!("{:?}", val),
            Value::CellPath(val) => val.into_string(),
            Value::CustomValue(val) => val.value_string(),
        }
    }

    /// Convert Value into string. Note that Streams will be consumed.
    pub fn into_abbreviated_string(&self, config: &Config) -> String {
        match self {
            Value::Bool(val) => val.to_string(),
            Value::Int(val) => val.to_string(),
            Value::Float(val) => val.to_string(),
            Value::Filesize(val) => format_filesize_from_conf(*val, config),
            Value::Duration(val) => format_duration(*val),
            Value::Date(val) => HumanTime::from(*val).to_string(),
            Value::Range(val) => {
                format!(
                    "{}..{}",
                    val.from.into_string(", ", config),
                    val.to.into_string(", ", config)
                )
            }
            Value::String(val) => val.to_string(),
            Value::List(ref vals) => match &vals[..] {
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
            Value::Nothing => String::new(),
            Value::Error(error) => format!("{:?}", error),
            Value::Binary(val) => format!("{:?}", val),
            Value::CellPath(val) => val.into_string(),
            Value::CustomValue(val) => val.value_string(),
        }
    }

    /// Convert Value into a debug string
    pub fn debug_value(&self) -> String {
        format!("{:#?}", self)
    }

    /// Convert Value into string. Note that Streams will be consumed.
    pub fn debug_string(&self, separator: &str, config: &Config) -> String {
        match self {
            Value::Bool(val) => val.to_string(),
            Value::Int(val) => val.to_string(),
            Value::Float(val) => val.to_string(),
            Value::Filesize(val) => format_filesize_from_conf(*val, config),
            Value::Duration(val) => format_duration(*val),
            Value::Date(val) => format!("{:?}", val),
            Value::Range(val) => {
                format!(
                    "{}..{}",
                    val.from.into_string(", ", config),
                    val.to.into_string(", ", config)
                )
            }
            Value::String(val) => val.clone(),
            Value::List(val) => format!(
                "[{}]",
                val.iter()
                    .map(|x| x.into_string(", ", config))
                    .collect::<Vec<_>>()
                    .join(separator)
            ),
            Value::Record { cols, vals } => format!(
                "{{{}}}",
                cols.iter()
                    .zip(vals.iter())
                    .map(|(x, y)| format!("{}: {}", x, y.into_string(", ", config)))
                    .collect::<Vec<_>>()
                    .join(separator)
            ),
            Value::Block { val, .. } => format!("<Block {}>", val),
            Value::Nothing => String::new(),
            Value::Error(error) => format!("{:?}", error),
            Value::Binary(val) => format!("{:?}", val),
            Value::CellPath(val) => val.into_string(),
            Value::CustomValue(val) => val.value_string(),
        }
    }

    /// Check if the content is empty
    pub fn is_empty(&self) -> bool {
        match self {
            Value::String(val) => val.is_empty(),
            Value::List(vals) => vals.is_empty(),
            Value::Record { cols, .. } => cols.is_empty(),
            Value::Binary(val) => val.is_empty(),
            Value::Nothing => true,
            _ => false,
        }
    }

    pub fn is_nothing(&self) -> bool {
        matches!(self, Value::Nothing)
    }

    /// Follow a given column path into the value: for example accessing select elements in a stream or list
    pub fn follow_cell_path(
        self,
        cell_path: &[PathMember],
        insensitive: bool,
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
                        Value::List(val) => {
                            if let Some(item) = val.get(*count) {
                                current = item.clone();
                            } else {
                                return Err(ShellError::AccessBeyondEnd(val.len(), *origin_span));
                            }
                        }
                        Value::Binary(val) => {
                            if let Some(item) = val.get(*count) {
                                current = Value::Int(*item as i64);
                            } else {
                                return Err(ShellError::AccessBeyondEnd(val.len(), *origin_span));
                            }
                        }
                        Value::Range(val) => {
                            if let Some(item) =
                                val.clone().into_range_iter(None, *origin_span)?.nth(*count)
                            {
                                current = item.clone();
                            } else {
                                return Err(ShellError::AccessBeyondEndOfStream(*origin_span));
                            }
                        }
                        Value::CustomValue(val) => {
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
                    Value::Record { cols, vals } => {
                        let cols = cols.clone();
                        let span = *origin_span;

                        // Make reverse iterate to avoid duplicate column leads to first value, actuall last value is expected.
                        if let Some(found) = cols.iter().zip(vals.iter()).rev().find(|x| {
                            if insensitive {
                                x.0.to_lowercase() == column_name.to_lowercase()
                            } else {
                                x.0 == column_name
                            }
                        }) {
                            current = found.1.clone();
                        } else if let Some(suggestion) = did_you_mean(&cols, column_name) {
                            return Err(ShellError::DidYouMean(suggestion, *origin_span));
                        } else {
                            return Err(ShellError::CantFindColumn(*origin_span));
                        }
                    }
                    Value::List(vals) => {
                        let mut output = vec![];
                        let mut hasvalue = false;
                        let mut temp: Result<Value, ShellError> =
                            Err(ShellError::NotFound(*origin_span));
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
                                output.push(Value::Nothing);
                            }
                        }
                        if hasvalue {
                            current = Value::List(output);
                        } else {
                            return temp;
                        }
                    }
                    Value::CustomValue(val) => {
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

    /// Follow a given column path into the value: for example accessing select elements in a stream or list
    pub fn upsert_cell_path(
        &mut self,
        cell_path: &[PathMember],
        callback: Box<dyn FnOnce(&Value) -> Value>,
    ) -> Result<(), ShellError> {
        let orig = self.clone();

        let new_val = callback(&orig.follow_cell_path(cell_path, false)?);

        match new_val {
            Value::Error(error) => Err(error),
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
                    Value::List(vals) => {
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
                                v => return Err(ShellError::CantFindColumn(*span)),
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
                                };
                                new_col.upsert_data_at_cell_path(&cell_path[1..], new_val)?;
                                vals.push(new_col);
                            }
                        }
                    }
                    v => return Err(ShellError::CantFindColumn(*span)),
                },
                PathMember::Int { val: row_num, span } => match self {
                    Value::List(vals) => {
                        if let Some(v) = vals.get_mut(*row_num) {
                            v.upsert_data_at_cell_path(&cell_path[1..], new_val)?
                        } else {
                            return Err(ShellError::AccessBeyondEnd(vals.len(), *span));
                        }
                    }
                    v => return Err(ShellError::NotAList(*span)),
                },
            },
            None => {
                *self = new_val;
            }
        }
        Ok(())
    }

    /// Follow a given column path into the value: for example accessing select elements in a stream or list
    pub fn update_cell_path(
        &mut self,
        cell_path: &[PathMember],
        callback: Box<dyn FnOnce(&Value) -> Value>,
    ) -> Result<(), ShellError> {
        let orig = self.clone();

        let new_val = callback(&orig.follow_cell_path(cell_path, false)?);

        match new_val {
            Value::Error(error) => Err(error),
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
                    Value::List(vals) => {
                        for val in vals.iter_mut() {
                            match val {
                                Value::Record { cols, vals } => {
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
                                        return Err(ShellError::CantFindColumn(*span));
                                    }
                                }
                                v => return Err(ShellError::CantFindColumn(*span)),
                            }
                        }
                    }
                    Value::Record { cols, vals } => {
                        let mut found = false;

                        for col in cols.iter().zip(vals.iter_mut()) {
                            if col.0 == col_name {
                                found = true;

                                col.1
                                    .update_data_at_cell_path(&cell_path[1..], new_val.clone())?
                            }
                        }
                        if !found {
                            return Err(ShellError::CantFindColumn(*span));
                        }
                    }
                    v => return Err(ShellError::CantFindColumn(*span)),
                },
                PathMember::Int { val: row_num, span } => match self {
                    Value::List(vals) => {
                        if let Some(v) = vals.get_mut(*row_num) {
                            v.update_data_at_cell_path(&cell_path[1..], new_val)?
                        } else {
                            return Err(ShellError::AccessBeyondEnd(vals.len(), *span));
                        }
                    }
                    v => return Err(ShellError::NotAList(*span)),
                },
            },
            None => {
                *self = new_val;
            }
        }
        Ok(())
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
                    Value::List(vals) => {
                        for val in vals.iter_mut() {
                            match val {
                                Value::Record { cols, vals } => {
                                    for col in cols.iter().zip(vals.iter_mut()) {
                                        if col.0 == col_name {
                                            if cell_path.len() == 1 {
                                                return Err(ShellError::ColumnAlreadyExists(*span));
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
                    Value::Record { cols, vals } => {
                        for col in cols.iter().zip(vals.iter_mut()) {
                            if col.0 == col_name {
                                if cell_path.len() == 1 {
                                    return Err(ShellError::ColumnAlreadyExists(*span));
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
                    Value::List(vals) => {
                        if let Some(v) = vals.get_mut(*row_num) {
                            v.insert_data_at_cell_path(&cell_path[1..], new_val)?
                        } else {
                            return Err(ShellError::AccessBeyondEnd(vals.len(), *span));
                        }
                    }
                    v => return Err(ShellError::NotAList(*span)),
                },
            },
            None => {
                *self = new_val;
            }
        }
        Ok(())
    }

    pub fn is_true(&self) -> bool {
        matches!(self, Value::Bool(true))
    }

    pub fn columns(&self) -> Vec<String> {
        match self {
            Value::Record { cols, .. } => cols.clone(),
            _ => vec![],
        }
    }

    pub fn string(val: impl Into<String>) -> Value {
        Value::String(val.into())
    }

    pub fn int(val: i64) -> Value {
        Value::Int(val)
    }

    pub fn float(val: f64) -> Value {
        Value::Float(val)
    }

    pub fn boolean(val: bool) -> Value {
        Value::Bool(val)
    }

    /// Note: Only use this for test data, *not* live data, as it will point into unknown source
    /// when used in errors.
    pub fn test_string(s: impl Into<String>) -> Value {
        Value::String(s.into())
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::Nothing
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
            (Value::Bool(lhs), rhs) => match rhs {
                Value::Bool(rhs) => lhs.partial_cmp(rhs),
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
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::Int(lhs), rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int(rhs) => lhs.partial_cmp(rhs),
                Value::Float(rhs) => compare_floats(*lhs as f64, *rhs),
                Value::Filesize { .. } => Some(Ordering::Less),
                Value::Duration { .. } => Some(Ordering::Less),
                Value::Date { .. } => Some(Ordering::Less),
                Value::Range { .. } => Some(Ordering::Less),
                Value::String { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Block { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::Float(lhs), rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int(rhs) => compare_floats(*lhs, *rhs as f64),
                Value::Float(rhs) => compare_floats(*lhs, *rhs),
                Value::Filesize { .. } => Some(Ordering::Less),
                Value::Duration { .. } => Some(Ordering::Less),
                Value::Date { .. } => Some(Ordering::Less),
                Value::Range { .. } => Some(Ordering::Less),
                Value::String { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Block { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::Filesize(lhs), rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize(rhs) => lhs.partial_cmp(rhs),
                Value::Duration { .. } => Some(Ordering::Less),
                Value::Date { .. } => Some(Ordering::Less),
                Value::Range { .. } => Some(Ordering::Less),
                Value::String { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Block { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::Duration(lhs), rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration(rhs) => lhs.partial_cmp(rhs),
                Value::Date { .. } => Some(Ordering::Less),
                Value::Range { .. } => Some(Ordering::Less),
                Value::String { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Block { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::Date(lhs), rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date(rhs) => lhs.partial_cmp(rhs),
                Value::Range { .. } => Some(Ordering::Less),
                Value::String { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Block { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::Range(lhs), rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { .. } => Some(Ordering::Greater),
                Value::Range(rhs) => lhs.partial_cmp(rhs),
                Value::String { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Block { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::String(lhs), rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { .. } => Some(Ordering::Greater),
                Value::Range { .. } => Some(Ordering::Greater),
                Value::String(rhs) => lhs.partial_cmp(rhs),
                Value::Record { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Block { .. } => Some(Ordering::Less),
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
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::List(lhs), rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { .. } => Some(Ordering::Greater),
                Value::Range { .. } => Some(Ordering::Greater),
                Value::String { .. } => Some(Ordering::Greater),
                Value::Record { .. } => Some(Ordering::Greater),
                Value::List(rhs) => lhs.partial_cmp(rhs),
                Value::Block { .. } => Some(Ordering::Less),
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
                Value::Nothing { .. } => Some(Ordering::Greater),
                Value::Error { .. } => Some(Ordering::Equal),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::Binary(lhs), rhs) => match rhs {
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
                Value::Nothing { .. } => Some(Ordering::Greater),
                Value::Error { .. } => Some(Ordering::Greater),
                Value::Binary(rhs) => lhs.partial_cmp(rhs),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::CellPath(lhs), rhs) => match rhs {
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
                Value::Nothing { .. } => Some(Ordering::Greater),
                Value::Error { .. } => Some(Ordering::Greater),
                Value::Binary { .. } => Some(Ordering::Greater),
                Value::CellPath(rhs) => lhs.partial_cmp(rhs),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::CustomValue(lhs), rhs) => lhs.partial_cmp(rhs),
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
            (Value::Int(lhs), Value::Int(rhs)) => {
                if let Some(val) = lhs.checked_add(*rhs) {
                    Ok(Value::Int(val))
                } else {
                    Err(ShellError::OperatorOverflow(
                        "add operation overflowed".into(),
                        span,
                    ))
                }
            }
            (Value::Int(lhs), Value::Float(rhs)) => Ok(Value::Float(*lhs as f64 + *rhs)),
            (Value::Float(lhs), Value::Int(rhs)) => Ok(Value::Float(*lhs + *rhs as f64)),
            (Value::Float(lhs), Value::Float(rhs)) => Ok(Value::Float(lhs + rhs)),
            (Value::String(lhs), Value::String(rhs)) => Ok(Value::String(lhs.to_string() + rhs)),
            (Value::Date(lhs), Value::Duration(rhs)) => {
                match lhs.checked_add_signed(chrono::Duration::nanoseconds(*rhs)) {
                    Some(val) => Ok(Value::Date(val)),
                    _ => Err(ShellError::OperatorOverflow(
                        "addition operation overflowed".into(),
                        span,
                    )),
                }
            }
            (Value::Duration(lhs), Value::Duration(rhs)) => {
                if let Some(val) = lhs.checked_add(*rhs) {
                    Ok(Value::Duration(val))
                } else {
                    Err(ShellError::OperatorOverflow(
                        "add operation overflowed".into(),
                        span,
                    ))
                }
            }
            (Value::Filesize(lhs), Value::Filesize(rhs)) => {
                if let Some(val) = lhs.checked_add(*rhs) {
                    Ok(Value::Filesize(val))
                } else {
                    Err(ShellError::OperatorOverflow(
                        "add operation overflowed".into(),
                        span,
                    ))
                }
            }

            (Value::CustomValue(lhs), rhs) => lhs.operation(span, Operator::Plus, op, rhs),

            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                rhs_ty: rhs.get_type(),
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
            (Value::Filesize { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if *rhs != 0 {
                    Ok(Value::Filesize {
                        val: lhs / rhs,
                        span,
                    })
                } else {
                    Err(ShellError::DivisionByZero(op))
                }
            }
            (Value::Duration { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if *rhs != 0 {
                    Ok(Value::Duration {
                        val: lhs / rhs,
                        span,
                    })
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
    pub fn lt(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        if let (Value::CustomValue { val: lhs, span }, rhs) = (self, rhs) {
            return lhs.operation(*span, Operator::LessThan, op, rhs);
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
            return lhs.operation(*span, Operator::LessThanOrEqual, op, rhs);
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
            return lhs.operation(*span, Operator::GreaterThan, op, rhs);
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
            return lhs.operation(*span, Operator::GreaterThanOrEqual, op, rhs);
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
    pub fn ne(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
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
                    val: if invert { !is_match } else { is_match },
                    span,
                })
            }
            (Value::CustomValue { val: lhs, span }, rhs) => lhs.operation(
                *span,
                if invert {
                    Operator::NotRegexMatch
                } else {
                    Operator::RegexMatch
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
                lhs.operation(*span, Operator::StartsWith, op, rhs)
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
                lhs.operation(*span, Operator::EndsWith, op, rhs)
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

    pub fn and(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
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

    pub fn or(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
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

fn format_filesize_from_conf(num_bytes: i64, config: &Config) -> String {
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

    let byte = byte_unit::Byte::from_bytes(num_bytes as u128);
    let adj_byte =
        if filesize_format_var.0 == byte_unit::ByteUnit::B && filesize_format_var.1 == "auto" {
            byte.get_appropriate_unit(!filesize_metric)
        } else {
            byte.get_adjusted_unit(filesize_format_var.0)
        };

    match adj_byte.get_unit() {
        byte_unit::ByteUnit::B => {
            let locale_string = get_locale().unwrap_or_else(|| String::from("en-US"));
            // Since get_locale() and Locale::from_name() don't always return the same items
            // we need to try and parse it to match. For instance, a valid locale is de_DE
            // however Locale::from_name() wants only de so we split and parse it out.
            let locale_string = locale_string.replace('_', "-"); // en_AU -> en-AU
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
}
