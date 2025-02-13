use super::{Expr, Expression};
use crate::{ShellError, Span};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Comparison {
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessThanOrEqual,
    GreaterThanOrEqual,
    RegexMatch,
    NotRegexMatch,
    In,
    NotIn,
    Has,
    NotHas,
    StartsWith,
    EndsWith,
}

impl Comparison {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Equal => "==",
            Self::NotEqual => "!=",
            Self::LessThan => "<",
            Self::GreaterThan => ">",
            Self::LessThanOrEqual => "<=",
            Self::GreaterThanOrEqual => ">=",
            Self::RegexMatch => "=~",
            Self::NotRegexMatch => "!~",
            Self::In => "in",
            Self::NotIn => "not-in",
            Self::Has => "has",
            Self::NotHas => "not-has",
            Self::StartsWith => "starts-with",
            Self::EndsWith => "ends-with",
        }
    }
}

impl fmt::Display for Comparison {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Math {
    Add,
    Subtract,
    Multiply,
    Divide,
    FloorDivide,
    Modulo,
    Pow,
    Concatenate,
}

impl Math {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::Divide => "/",
            Self::FloorDivide => "//",
            Self::Modulo => "mod",
            Self::Pow => "**",
            Self::Concatenate => "++",
        }
    }
}

impl fmt::Display for Math {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Boolean {
    Or,
    Xor,
    And,
}

impl Boolean {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Or => "or",
            Self::Xor => "xor",
            Self::And => "and",
        }
    }
}

impl fmt::Display for Boolean {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Bits {
    BitOr,
    BitXor,
    BitAnd,
    ShiftLeft,
    ShiftRight,
}

impl Bits {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::BitOr => "bit-or",
            Self::BitXor => "bit-xor",
            Self::BitAnd => "bit-and",
            Self::ShiftLeft => "bit-shl",
            Self::ShiftRight => "bit-shr",
        }
    }
}

impl fmt::Display for Bits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Assignment {
    Assign,
    AddAssign,
    SubtractAssign,
    MultiplyAssign,
    DivideAssign,
    ConcatenateAssign,
}

impl Assignment {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Assign => "=",
            Self::AddAssign => "+=",
            Self::SubtractAssign => "-=",
            Self::MultiplyAssign => "*=",
            Self::DivideAssign => "/=",
            Self::ConcatenateAssign => "++=",
        }
    }
}

impl fmt::Display for Assignment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Operator {
    Comparison(Comparison),
    Math(Math),
    Boolean(Boolean),
    Bits(Bits),
    Assignment(Assignment),
}

impl Operator {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Comparison(comparison) => comparison.as_str(),
            Self::Math(math) => math.as_str(),
            Self::Boolean(boolean) => boolean.as_str(),
            Self::Bits(bits) => bits.as_str(),
            Self::Assignment(assignment) => assignment.as_str(),
        }
    }

    pub const fn precedence(&self) -> u8 {
        match self {
            Self::Math(Math::Pow) => 100,
            Self::Math(Math::Multiply)
            | Self::Math(Math::Divide)
            | Self::Math(Math::Modulo)
            | Self::Math(Math::FloorDivide) => 95,
            Self::Math(Math::Add) | Self::Math(Math::Subtract) => 90,
            Self::Bits(Bits::ShiftLeft) | Self::Bits(Bits::ShiftRight) => 85,
            Self::Comparison(Comparison::NotRegexMatch)
            | Self::Comparison(Comparison::RegexMatch)
            | Self::Comparison(Comparison::StartsWith)
            | Self::Comparison(Comparison::EndsWith)
            | Self::Comparison(Comparison::LessThan)
            | Self::Comparison(Comparison::LessThanOrEqual)
            | Self::Comparison(Comparison::GreaterThan)
            | Self::Comparison(Comparison::GreaterThanOrEqual)
            | Self::Comparison(Comparison::Equal)
            | Self::Comparison(Comparison::NotEqual)
            | Self::Comparison(Comparison::In)
            | Self::Comparison(Comparison::NotIn)
            | Self::Comparison(Comparison::Has)
            | Self::Comparison(Comparison::NotHas)
            | Self::Math(Math::Concatenate) => 80,
            Self::Bits(Bits::BitAnd) => 75,
            Self::Bits(Bits::BitXor) => 70,
            Self::Bits(Bits::BitOr) => 60,
            Self::Boolean(Boolean::And) => 50,
            Self::Boolean(Boolean::Xor) => 45,
            Self::Boolean(Boolean::Or) => 40,
            Self::Assignment(_) => 10,
        }
    }
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RangeInclusion {
    Inclusive,
    RightExclusive,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RangeOperator {
    pub inclusion: RangeInclusion,
    pub span: Span,
    pub next_op_span: Span,
}

impl fmt::Display for RangeOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.inclusion {
            RangeInclusion::Inclusive => write!(f, ".."),
            RangeInclusion::RightExclusive => write!(f, "..<"),
        }
    }
}

pub fn eval_operator(op: &Expression) -> Result<Operator, ShellError> {
    match op {
        Expression {
            expr: Expr::Operator(operator),
            ..
        } => Ok(*operator),
        Expression { span, expr, .. } => Err(ShellError::UnknownOperator {
            op_token: format!("{expr:?}"),
            span: *span,
        }),
    }
}
