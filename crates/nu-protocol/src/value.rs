use std::{cell::RefCell, fmt::Debug, rc::Rc};

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
    Float {
        val: f64,
        span: Span,
    },
    String {
        val: String,
        span: Span,
    },
    ValueStream {
        stream: ValueStream,
        span: Span,
    },
    RowStream {
        headers: Vec<String>,
        stream: RowStream,
        span: Span,
    },
    List {
        val: Vec<Value>,
        span: Span,
    },
    Table {
        headers: Vec<String>,
        val: Vec<Vec<Value>>,
        span: Span,
    },
    Block {
        val: BlockId,
        span: Span,
    },
    Nothing {
        span: Span,
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
            Value::String { span, .. } => *span,
            Value::List { span, .. } => *span,
            Value::Table { span, .. } => *span,
            Value::Block { span, .. } => *span,
            Value::RowStream { span, .. } => *span,
            Value::ValueStream { span, .. } => *span,
            Value::Nothing { span, .. } => *span,
        }
    }

    pub fn with_span(mut self, new_span: Span) -> Value {
        match &mut self {
            Value::Bool { span, .. } => *span = new_span,
            Value::Int { span, .. } => *span = new_span,
            Value::Float { span, .. } => *span = new_span,
            Value::String { span, .. } => *span = new_span,
            Value::RowStream { span, .. } => *span = new_span,
            Value::ValueStream { span, .. } => *span = new_span,
            Value::List { span, .. } => *span = new_span,
            Value::Table { span, .. } => *span = new_span,
            Value::Block { span, .. } => *span = new_span,
            Value::Nothing { span, .. } => *span = new_span,
        }

        self
    }

    pub fn get_type(&self) -> Type {
        match self {
            Value::Bool { .. } => Type::Bool,
            Value::Int { .. } => Type::Int,
            Value::Float { .. } => Type::Float,
            Value::String { .. } => Type::String,
            Value::List { .. } => Type::List(Box::new(Type::Unknown)), // FIXME
            Value::Table { .. } => Type::Table,                        // FIXME
            Value::Nothing { .. } => Type::Nothing,
            Value::Block { .. } => Type::Block,
            Value::ValueStream { .. } => Type::ValueStream,
            Value::RowStream { .. } => Type::RowStream,
        }
    }

    pub fn into_string(self) -> String {
        match self {
            Value::Bool { val, .. } => val.to_string(),
            Value::Int { val, .. } => val.to_string(),
            Value::Float { val, .. } => val.to_string(),
            Value::String { val, .. } => val,
            Value::ValueStream { stream, .. } => stream.into_string(),
            Value::List { val, .. } => val
                .into_iter()
                .map(|x| x.into_string())
                .collect::<Vec<_>>()
                .join(", "),
            Value::Table { val, .. } => val
                .into_iter()
                .map(|x| {
                    x.into_iter()
                        .map(|x| x.into_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .collect::<Vec<_>>()
                .join("\n"),
            Value::RowStream {
                headers, stream, ..
            } => stream.into_string(headers),
            Value::Block { val, .. } => format!("<Block {}>", val),
            Value::Nothing { .. } => String::new(),
        }
    }

    pub fn nothing() -> Value {
        Value::Nothing {
            span: Span::unknown(),
        }
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
                    Ok(Value::Int {
                        val: lhs / rhs,
                        span,
                    })
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
            (Value::List { val: lhs, .. }, Value::List { val: rhs, .. }) => Ok(Value::Bool {
                val: lhs == rhs,
                span,
            }),
            (
                Value::Table {
                    val: lhs,
                    headers: lhs_headers,
                    ..
                },
                Value::Table {
                    val: rhs,
                    headers: rhs_headers,
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
            (Value::List { val: lhs, .. }, Value::List { val: rhs, .. }) => Ok(Value::Bool {
                val: lhs != rhs,
                span,
            }),
            (
                Value::Table {
                    val: lhs,
                    headers: lhs_headers,
                    ..
                },
                Value::Table {
                    val: rhs,
                    headers: rhs_headers,
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
