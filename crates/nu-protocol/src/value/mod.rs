mod range;
mod row;
mod stream;
mod unit;

use chrono::{DateTime, FixedOffset};
use chrono_humanize::HumanTime;
pub use range::*;
pub use row::*;
use serde::{Deserialize, Serialize};
pub use stream::*;
pub use unit::*;

use std::collections::HashMap;
use std::{cmp::Ordering, fmt::Debug};

use crate::ast::{CellPath, PathMember};
use crate::{span, BlockId, Span, Spanned, Type};

use crate::ShellError;

/// Core structured values that pass through the pipeline in engine-q
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

impl Value {
    pub fn as_string(&self) -> Result<String, ShellError> {
        match self {
            Value::String { val, .. } => Ok(val.to_string()),
            _ => Err(ShellError::CantConvert("string".into(), self.span()?)),
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
            Value::Record { cols, vals, .. } => {
                Type::Record(cols.clone(), vals.iter().map(|x| x.get_type()).collect())
            }
            Value::List { .. } => Type::List(Box::new(Type::Unknown)), // FIXME
            Value::Nothing { .. } => Type::Nothing,
            Value::Block { .. } => Type::Block,
            Value::Error { .. } => Type::Error,
            Value::Binary { .. } => Type::Binary,
            Value::CellPath { .. } => Type::CellPath,
        }
    }

    /// Convert Value into string. Note that Streams will be consumed.
    pub fn into_string(self) -> String {
        match self {
            Value::Bool { val, .. } => val.to_string(),
            Value::Int { val, .. } => val.to_string(),
            Value::Float { val, .. } => val.to_string(),
            Value::Filesize { val, .. } => format_filesize(val),
            Value::Duration { val, .. } => format_duration(val),
            Value::Date { val, .. } => HumanTime::from(val).to_string(),
            Value::Range { val, .. } => {
                format!("{}..{}", val.from.into_string(), val.to.into_string())
            }
            Value::String { val, .. } => val,
            Value::List { vals: val, .. } => format!(
                "[{}]",
                val.into_iter()
                    .map(|x| x.into_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::Record { cols, vals, .. } => format!(
                "{{{}}}",
                cols.iter()
                    .zip(vals.iter())
                    .map(|(x, y)| format!("{}: {}", x, y.clone().into_string()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::Block { val, .. } => format!("<Block {}>", val),
            Value::Nothing { .. } => String::new(),
            Value::Error { error } => format!("{:?}", error),
            Value::Binary { val, .. } => format!("{:?}", val),
            Value::CellPath { val, .. } => val.into_string(),
        }
    }

    pub fn collect_string(self) -> String {
        match self {
            Value::Bool { val, .. } => val.to_string(),
            Value::Int { val, .. } => val.to_string(),
            Value::Float { val, .. } => val.to_string(),
            Value::Filesize { val, .. } => format_filesize(val),
            Value::Duration { val, .. } => format_duration(val),
            Value::Date { val, .. } => format!("{:?}", val),
            Value::Range { val, .. } => {
                format!("{}..{}", val.from.into_string(), val.to.into_string())
            }
            Value::String { val, .. } => val,
            Value::List { vals: val, .. } => val
                .into_iter()
                .map(|x| x.collect_string())
                .collect::<Vec<_>>()
                .join("\n"),
            Value::Record { vals, .. } => vals
                .into_iter()
                .map(|y| y.collect_string())
                .collect::<Vec<_>>()
                .join("\n"),
            Value::Block { val, .. } => format!("<Block {}>", val),
            Value::Nothing { .. } => String::new(),
            Value::Error { error } => format!("{:?}", error),
            Value::Binary { val, .. } => format!("{:?}", val),
            Value::CellPath { .. } => self.into_string(),
        }
    }

    /// Create a new `Nothing` value
    pub fn nothing() -> Value {
        Value::Nothing {
            span: Span::unknown(),
        }
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
                        let span = *span;
                        let mut found = false;
                        for col in cols.iter().zip(vals.iter()) {
                            if col.0 == column_name {
                                current = col.1.clone();
                                found = true;
                                break;
                            }
                        }

                        if !found {
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

    // Only use these for test data. Span::unknown() should not be used in user data
    pub fn test_string(s: impl Into<String>) -> Value {
        Value::String {
            val: s.into(),
            span: Span::unknown(),
        }
    }

    // Only use these for test data. Span::unknown() should not be used in user data
    pub fn test_int(val: i64) -> Value {
        Value::Int {
            val,
            span: Span::unknown(),
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::nothing()
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
            (Value::Date { val: lhs, .. }, Value::Date { val: rhs, .. }) => {
                lhs.date().to_string().partial_cmp(&rhs.date().to_string())
            }
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
            (Value::Duration { val: lhs, .. }, Value::Duration { val: rhs, .. }) => {
                Ok(Value::Duration {
                    val: *lhs + *rhs,
                    span,
                })
            }
            (Value::Filesize { val: lhs, .. }, Value::Filesize { val: rhs, .. }) => {
                Ok(Value::Filesize {
                    val: *lhs + *rhs,
                    span,
                })
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
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Int {
                val: lhs - rhs,
                span,
            }),
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
            (Value::Duration { val: lhs, .. }, Value::Duration { val: rhs, .. }) => {
                Ok(Value::Duration {
                    val: *lhs - *rhs,
                    span,
                })
            }
            (Value::Filesize { val: lhs, .. }, Value::Filesize { val: rhs, .. }) => {
                Ok(Value::Filesize {
                    val: *lhs - *rhs,
                    span,
                })
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
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Int {
                val: lhs * rhs,
                span,
            }),
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

        match self.partial_cmp(rhs) {
            Some(ordering) => Ok(Value::Bool {
                val: matches!(ordering, Ordering::Equal),
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
    pub fn ne(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span()?, rhs.span()?]);

        match self.partial_cmp(rhs) {
            Some(ordering) => Ok(Value::Bool {
                val: !matches!(ordering, Ordering::Equal),
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
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Int {
                val: lhs.pow(*rhs as u32),
                span,
            }),
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

fn format_filesize(num_bytes: i64) -> String {
    let byte = byte_unit::Byte::from_bytes(num_bytes as u128);

    if byte.get_bytes() == 0u128 {
        return "—".to_string();
    }

    let byte = byte.get_appropriate_unit(false);

    match byte.get_unit() {
        byte_unit::ByteUnit::B => format!("{} B ", byte.get_value()),
        _ => byte.format(1),
    }
}
