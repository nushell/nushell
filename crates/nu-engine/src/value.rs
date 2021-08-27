use std::fmt::Display;

use nu_parser::{BlockId, Span, Type};

use crate::ShellError;

#[derive(Debug, Clone)]
pub enum Value {
    Bool { val: bool, span: Span },
    Int { val: i64, span: Span },
    Float { val: f64, span: Span },
    String { val: String, span: Span },
    List { val: Vec<Value>, span: Span },
    Block { val: BlockId, span: Span },
    Nothing { span: Span },
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
            Value::Block { span, .. } => *span,
            Value::Nothing { span, .. } => *span,
        }
    }

    pub fn with_span(mut self, new_span: Span) -> Value {
        match &mut self {
            Value::Bool { span, .. } => *span = new_span,
            Value::Int { span, .. } => *span = new_span,
            Value::Float { span, .. } => *span = new_span,
            Value::String { span, .. } => *span = new_span,
            Value::List { span, .. } => *span = new_span,
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
            Value::Nothing { .. } => Type::Nothing,
            Value::Block { .. } => Type::Block,
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
            (Value::List { val: l1, .. }, Value::List { val: l2, .. }) => l1 == l2,
            (Value::Block { val: b1, .. }, Value::Block { val: b2, .. }) => b1 == b2,
            _ => false,
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Bool { val, .. } => {
                write!(f, "{}", val)
            }
            Value::Int { val, .. } => {
                write!(f, "{}", val)
            }
            Value::Float { val, .. } => {
                write!(f, "{}", val)
            }
            Value::String { val, .. } => write!(f, "{}", val),
            Value::List { val, .. } => {
                write!(
                    f,
                    "[{}]",
                    val.iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<String>>()
                        .join(", ".into())
                )
            }
            Value::Block { .. } => write!(f, "<block>"),
            Value::Nothing { .. } => write!(f, ""),
        }
    }
}

impl Value {
    pub fn add(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = nu_parser::span(&[self.span(), rhs.span()]);

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
        let span = nu_parser::span(&[self.span(), rhs.span()]);

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
        let span = nu_parser::span(&[self.span(), rhs.span()]);

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
        let span = nu_parser::span(&[self.span(), rhs.span()]);

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
        let span = nu_parser::span(&[self.span(), rhs.span()]);

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
        let span = nu_parser::span(&[self.span(), rhs.span()]);

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
        let span = nu_parser::span(&[self.span(), rhs.span()]);

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
        let span = nu_parser::span(&[self.span(), rhs.span()]);

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
        let span = nu_parser::span(&[self.span(), rhs.span()]);

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
        let span = nu_parser::span(&[self.span(), rhs.span()]);

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
