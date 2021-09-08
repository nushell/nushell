mod range;
mod row;
mod stream;

pub use range::*;
pub use row::*;
pub use stream::*;

use std::{cell::RefCell, fmt::Debug, rc::Rc};

use crate::ast::{PathMember, RangeInclusion};
use crate::{span, BlockId, Span, Type};

use crate::ShellError;

/// Core structured values that pass through the pipeline in engine-q
#[derive(Debug, Clone)]
pub enum Value {
    Bool {
        val: bool,
        span: Span,
    },
    Int {
        val: i64,
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
    Stream {
        stream: ValueStream,
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
}

impl Value {
    pub fn as_string(&self) -> Result<String, ShellError> {
        match self {
            Value::String { val, .. } => Ok(val.to_string()),
            _ => Err(ShellError::CantConvert("string".into(), self.span())),
        }
    }

    /// Get the span for the current value
    pub fn span(&self) -> Span {
        match self {
            Value::Error { .. } => Span::unknown(),
            Value::Bool { span, .. } => *span,
            Value::Int { span, .. } => *span,
            Value::Float { span, .. } => *span,
            Value::Range { span, .. } => *span,
            Value::String { span, .. } => *span,
            Value::Record { span, .. } => *span,
            Value::List { span, .. } => *span,
            Value::Block { span, .. } => *span,
            Value::Stream { span, .. } => *span,
            Value::Nothing { span, .. } => *span,
        }
    }

    /// Update the value with a new span
    pub fn with_span(mut self, new_span: Span) -> Value {
        match &mut self {
            Value::Bool { span, .. } => *span = new_span,
            Value::Int { span, .. } => *span = new_span,
            Value::Float { span, .. } => *span = new_span,
            Value::Range { span, .. } => *span = new_span,
            Value::String { span, .. } => *span = new_span,
            Value::Record { span, .. } => *span = new_span,
            Value::Stream { span, .. } => *span = new_span,
            Value::List { span, .. } => *span = new_span,
            Value::Block { span, .. } => *span = new_span,
            Value::Nothing { span, .. } => *span = new_span,
            Value::Error { .. } => {}
        }

        self
    }

    /// Get the type of the current Value
    pub fn get_type(&self) -> Type {
        match self {
            Value::Bool { .. } => Type::Bool,
            Value::Int { .. } => Type::Int,
            Value::Float { .. } => Type::Float,
            Value::Range { .. } => Type::Range,
            Value::String { .. } => Type::String,
            Value::Record { cols, vals, .. } => {
                Type::Record(cols.clone(), vals.iter().map(|x| x.get_type()).collect())
            }
            Value::List { .. } => Type::List(Box::new(Type::Unknown)), // FIXME
            Value::Nothing { .. } => Type::Nothing,
            Value::Block { .. } => Type::Block,
            Value::Stream { .. } => Type::ValueStream,
            Value::Error { .. } => Type::Error,
        }
    }

    /// Convert Value into string. Note that Streams will be consumed.
    pub fn into_string(self) -> String {
        match self {
            Value::Bool { val, .. } => val.to_string(),
            Value::Int { val, .. } => val.to_string(),
            Value::Float { val, .. } => val.to_string(),
            Value::Range { val, .. } => {
                let vals: Vec<i64> = match (&val.from, &val.to) {
                    (Value::Int { val: from, .. }, Value::Int { val: to, .. }) => {
                        match val.inclusion {
                            RangeInclusion::Inclusive => (*from..=*to).collect(),
                            RangeInclusion::RightExclusive => (*from..*to).collect(),
                        }
                    }
                    _ => Vec::new(),
                };

                format!(
                    "range: [{}]",
                    vals.iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<String>>()
                        .join(", ")
                )
            }
            Value::String { val, .. } => val,
            Value::Stream { stream, .. } => stream.into_string(),
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
        }
    }

    /// Create a new `Nothing` value
    pub fn nothing() -> Value {
        Value::Nothing {
            span: Span::unknown(),
        }
    }

    pub fn follow_cell_path(self, column_path: &[PathMember]) -> Result<Value, ShellError> {
        let mut current = self;
        for member in column_path {
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
                        Value::Stream { stream, .. } => {
                            if let Some(item) = stream.nth(*count) {
                                current = item;
                            } else {
                                return Err(ShellError::AccessBeyondEndOfStream(*origin_span));
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
                    Value::Record { cols, vals, .. } => {
                        let mut found = false;
                        for col in cols.iter().zip(vals.iter()) {
                            if col.0 == column_name {
                                current = col.1.clone();
                                found = true;
                                break;
                            }
                        }

                        if !found {
                            return Err(ShellError::CantFindColumn(*origin_span));
                        }
                    }
                    Value::List { vals, span } => {
                        let mut output = vec![];
                        for val in vals {
                            if let Value::Record { cols, vals, .. } = val {
                                for col in cols.iter().enumerate() {
                                    if col.1 == column_name {
                                        output.push(vals[col.0].clone());
                                    }
                                }
                            }
                        }

                        current = Value::List {
                            vals: output,
                            span: *span,
                        };
                    }
                    Value::Stream { stream, span } => {
                        let mut output = vec![];
                        for val in stream {
                            if let Value::Record { cols, vals, .. } = val {
                                for col in cols.iter().enumerate() {
                                    if col.1 == column_name {
                                        output.push(vals[col.0].clone());
                                    }
                                }
                            }
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
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Bool { val: lhs, .. }, Value::Bool { val: rhs, .. }) => lhs == rhs,
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => lhs == rhs,
            (Value::Float { val: lhs, .. }, Value::Float { val: rhs, .. }) => lhs == rhs,
            (Value::String { val: lhs, .. }, Value::String { val: rhs, .. }) => lhs == rhs,
            (Value::Block { val: b1, .. }, Value::Block { val: b2, .. }) => b1 == b2,
            _ => false,
        }
    }
}

impl Value {
    pub fn add(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span(), rhs.span()]);

        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Int {
                val: lhs + rhs,
                span,
            }),
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

            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span(),
            }),
        }
    }
    pub fn sub(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span(), rhs.span()]);

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

            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span(),
            }),
        }
    }
    pub fn mul(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span(), rhs.span()]);

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
                lhs_span: self.span(),
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span(),
            }),
        }
    }
    pub fn div(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span(), rhs.span()]);

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
                lhs_span: self.span(),
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span(),
            }),
        }
    }
    pub fn lt(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span(), rhs.span()]);

        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Bool {
                val: lhs < rhs,
                span,
            }),
            (Value::Int { val: lhs, .. }, Value::Float { val: rhs, .. }) => Ok(Value::Bool {
                val: (*lhs as f64) < *rhs,
                span,
            }),
            (Value::Float { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Bool {
                val: *lhs < *rhs as f64,
                span,
            }),
            (Value::Float { val: lhs, .. }, Value::Float { val: rhs, .. }) => Ok(Value::Bool {
                val: lhs < rhs,
                span,
            }),
            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span(),
            }),
        }
    }
    pub fn lte(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span(), rhs.span()]);

        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Bool {
                val: lhs <= rhs,
                span,
            }),
            (Value::Int { val: lhs, .. }, Value::Float { val: rhs, .. }) => Ok(Value::Bool {
                val: (*lhs as f64) <= *rhs,
                span,
            }),
            (Value::Float { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Bool {
                val: *lhs <= *rhs as f64,
                span,
            }),
            (Value::Float { val: lhs, .. }, Value::Float { val: rhs, .. }) => Ok(Value::Bool {
                val: lhs <= rhs,
                span,
            }),
            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span(),
            }),
        }
    }
    pub fn gt(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span(), rhs.span()]);

        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Bool {
                val: lhs > rhs,
                span,
            }),
            (Value::Int { val: lhs, .. }, Value::Float { val: rhs, .. }) => Ok(Value::Bool {
                val: (*lhs as f64) > *rhs,
                span,
            }),
            (Value::Float { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Bool {
                val: *lhs > *rhs as f64,
                span,
            }),
            (Value::Float { val: lhs, .. }, Value::Float { val: rhs, .. }) => Ok(Value::Bool {
                val: lhs > rhs,
                span,
            }),
            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span(),
            }),
        }
    }
    pub fn gte(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span(), rhs.span()]);

        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Bool {
                val: lhs >= rhs,
                span,
            }),
            (Value::Int { val: lhs, .. }, Value::Float { val: rhs, .. }) => Ok(Value::Bool {
                val: (*lhs as f64) >= *rhs,
                span,
            }),
            (Value::Float { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Bool {
                val: *lhs >= *rhs as f64,
                span,
            }),
            (Value::Float { val: lhs, .. }, Value::Float { val: rhs, .. }) => Ok(Value::Bool {
                val: lhs >= rhs,
                span,
            }),
            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span(),
            }),
        }
    }
    pub fn eq(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span(), rhs.span()]);

        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Bool {
                val: lhs == rhs,
                span,
            }),
            (Value::String { val: lhs, .. }, Value::String { val: rhs, .. }) => Ok(Value::Bool {
                val: lhs == rhs,
                span,
            }),
            // FIXME: these should consider machine epsilon
            (Value::Int { val: lhs, .. }, Value::Float { val: rhs, .. }) => Ok(Value::Bool {
                val: (*lhs as f64) == *rhs,
                span,
            }),
            // FIXME: these should consider machine epsilon
            (Value::Float { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Bool {
                val: *lhs == *rhs as f64,
                span,
            }),
            // FIXME: these should consider machine epsilon
            (Value::Float { val: lhs, .. }, Value::Float { val: rhs, .. }) => Ok(Value::Bool {
                val: lhs == rhs,
                span,
            }),
            (Value::List { vals: lhs, .. }, Value::List { vals: rhs, .. }) => Ok(Value::Bool {
                val: lhs == rhs,
                span,
            }),
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
            ) => Ok(Value::Bool {
                val: lhs_headers == rhs_headers && lhs == rhs,
                span,
            }),
            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span(),
            }),
        }
    }
    pub fn ne(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span(), rhs.span()]);

        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Bool {
                val: lhs != rhs,
                span,
            }),
            (Value::String { val: lhs, .. }, Value::String { val: rhs, .. }) => Ok(Value::Bool {
                val: lhs != rhs,
                span,
            }),
            // FIXME: these should consider machine epsilon
            (Value::Int { val: lhs, .. }, Value::Float { val: rhs, .. }) => Ok(Value::Bool {
                val: (*lhs as f64) != *rhs,
                span,
            }),
            // FIXME: these should consider machine epsilon
            (Value::Float { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Bool {
                val: *lhs != *rhs as f64,
                span,
            }),
            // FIXME: these should consider machine epsilon
            (Value::Float { val: lhs, .. }, Value::Float { val: rhs, .. }) => Ok(Value::Bool {
                val: lhs != rhs,
                span,
            }),
            (Value::List { vals: lhs, .. }, Value::List { vals: rhs, .. }) => Ok(Value::Bool {
                val: lhs != rhs,
                span,
            }),
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
            ) => Ok(Value::Bool {
                val: lhs_headers != rhs_headers || lhs != rhs,
                span,
            }),

            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span(),
            }),
        }
    }
}
