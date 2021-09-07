use std::{cell::RefCell, fmt::Debug, rc::Rc};

use crate::ast::{PathMember, RangeInclusion};
use crate::{span, BlockId, Span, Type};

use crate::ShellError;

#[derive(Clone)]
pub struct ValueStream(pub Rc<RefCell<dyn Iterator<Item = Value>>>);

impl ValueStream {
    pub fn into_string(self) -> String {
        format!(
            "[{}]",
            self.map(|x| x.into_string())
                .collect::<Vec<String>>()
                .join(", ")
        )
    }

    pub fn from_stream(input: impl Iterator<Item = Value> + 'static) -> ValueStream {
        ValueStream(Rc::new(RefCell::new(input)))
    }
}

impl Debug for ValueStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValueStream").finish()
    }
}

impl Iterator for ValueStream {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        {
            let mut iter = self.0.borrow_mut();
            iter.next()
        }
    }
}

pub trait IntoValueStream {
    fn into_value_stream(self) -> ValueStream;
}

impl<T> IntoValueStream for T
where
    T: Iterator<Item = Value> + 'static,
{
    fn into_value_stream(self) -> ValueStream {
        ValueStream::from_stream(self)
    }
}

#[derive(Clone)]
pub struct RowStream(Rc<RefCell<dyn Iterator<Item = Vec<Value>>>>);

impl RowStream {
    pub fn into_string(self, headers: Vec<String>) -> String {
        format!(
            "[{}]\n[{}]",
            headers
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<String>>()
                .join(", "),
            self.map(|x| {
                x.into_iter()
                    .map(|x| x.into_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            })
            .collect::<Vec<String>>()
            .join("\n")
        )
    }
}

impl Debug for RowStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValueStream").finish()
    }
}

impl Iterator for RowStream {
    type Item = Vec<Value>;

    fn next(&mut self) -> Option<Self::Item> {
        {
            let mut iter = self.0.borrow_mut();
            iter.next()
        }
    }
}

pub trait IntoRowStream {
    fn into_row_stream(self) -> RowStream;
}

impl IntoRowStream for Vec<Vec<Value>> {
    fn into_row_stream(self) -> RowStream {
        RowStream(Rc::new(RefCell::new(self.into_iter())))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Range {
    pub from: Value,
    pub to: Value,
    pub inclusion: RangeInclusion,
}

impl IntoIterator for Range {
    type Item = Value;

    type IntoIter = RangeIterator;

    fn into_iter(self) -> Self::IntoIter {
        let span = self.from.span();

        RangeIterator::new(self, span)
    }
}

pub struct RangeIterator {
    curr: Value,
    end: Value,
    span: Span,
    is_end_inclusive: bool,
    moves_up: bool,
    one: Value,
    negative_one: Value,
    done: bool,
}

impl RangeIterator {
    pub fn new(range: Range, span: Span) -> RangeIterator {
        let start = match range.from {
            Value::Nothing { .. } => Value::Int { val: 0, span },
            x => x,
        };

        let end = match range.to {
            Value::Nothing { .. } => Value::Int {
                val: i64::MAX,
                span,
            },
            x => x,
        };

        RangeIterator {
            moves_up: matches!(start.lte(span, &end), Ok(Value::Bool { val: true, .. })),
            curr: start,
            end,
            span,
            is_end_inclusive: matches!(range.inclusion, RangeInclusion::Inclusive),
            done: false,
            one: Value::Int { val: 1, span },
            negative_one: Value::Int { val: -1, span },
        }
    }
}

impl Iterator for RangeIterator {
    type Item = Value;
    fn next(&mut self) -> Option<Self::Item> {
        use std::cmp::Ordering;
        if self.done {
            return None;
        }

        let ordering = if matches!(self.end, Value::Nothing { .. }) {
            Ordering::Less
        } else {
            match (&self.curr, &self.end) {
                (Value::Int { val: x, .. }, Value::Int { val: y, .. }) => x.cmp(y),
                // (Value::Float { val: x, .. }, Value::Float { val: y, .. }) => x.cmp(y),
                // (Value::Float { val: x, .. }, Value::Int { val: y, .. }) => x.cmp(y),
                // (Value::Int { val: x, .. }, Value::Float { val: y, .. }) => x.cmp(y),
                _ => {
                    self.done = true;
                    return Some(Value::Error {
                        error: ShellError::CannotCreateRange(self.span),
                    });
                }
            }
        };

        if self.moves_up
            && (ordering == Ordering::Less || self.is_end_inclusive && ordering == Ordering::Equal)
        {
            let next_value = self.curr.add(self.span, &self.one);

            let mut next = match next_value {
                Ok(result) => result,

                Err(error) => {
                    self.done = true;
                    return Some(Value::Error { error });
                }
            };
            std::mem::swap(&mut self.curr, &mut next);

            Some(next)
        } else if !self.moves_up
            && (ordering == Ordering::Greater
                || self.is_end_inclusive && ordering == Ordering::Equal)
        {
            let next_value = self.curr.add(self.span, &self.negative_one);

            let mut next = match next_value {
                Ok(result) => result,
                Err(error) => {
                    self.done = true;
                    return Some(Value::Error { error });
                }
            };
            std::mem::swap(&mut self.curr, &mut next);

            Some(next)
        } else {
            None
        }
    }
}

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
    ValueStream {
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

    pub fn span(&self) -> Span {
        match self {
            Value::Bool { span, .. } => *span,
            Value::Int { span, .. } => *span,
            Value::Float { span, .. } => *span,
            Value::Range { span, .. } => *span,
            Value::String { span, .. } => *span,
            Value::Record { span, .. } => *span,
            Value::List { span, .. } => *span,
            Value::Block { span, .. } => *span,
            Value::ValueStream { span, .. } => *span,
            Value::Nothing { span, .. } => *span,
            Value::Error { .. } => Span::unknown(),
        }
    }

    pub fn with_span(mut self, new_span: Span) -> Value {
        match &mut self {
            Value::Bool { span, .. } => *span = new_span,
            Value::Int { span, .. } => *span = new_span,
            Value::Float { span, .. } => *span = new_span,
            Value::Range { span, .. } => *span = new_span,
            Value::String { span, .. } => *span = new_span,
            Value::Record { span, .. } => *span = new_span,
            Value::ValueStream { span, .. } => *span = new_span,
            Value::List { span, .. } => *span = new_span,
            Value::Block { span, .. } => *span = new_span,
            Value::Nothing { span, .. } => *span = new_span,
            Value::Error { .. } => {}
        }

        self
    }

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
            Value::ValueStream { .. } => Type::ValueStream,
            Value::Error { .. } => Type::Error,
        }
    }

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
            Value::ValueStream { stream, .. } => stream.into_string(),
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
                        Value::ValueStream { stream, .. } => {
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
