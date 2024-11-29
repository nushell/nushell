use crate::{ShellError, Span};

use serde::{Deserialize, Serialize};
use std::fmt::Display;

use super::{Expr, Expression};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    StartsWith,
    EndsWith,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Math {
    Plus,
    Concat,
    Minus,
    Multiply,
    Divide,
    Modulo,
    FloorDivision,
    Pow,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Boolean {
    And,
    Or,
    Xor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Bits {
    BitOr,
    BitXor,
    BitAnd,
    ShiftLeft,
    ShiftRight,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Assignment {
    Assign,
    PlusAssign,
    ConcatAssign,
    MinusAssign,
    MultiplyAssign,
    DivideAssign,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Operator {
    Comparison(Comparison),
    Math(Math),
    Boolean(Boolean),
    Bits(Bits),
    Assignment(Assignment),
}

impl Operator {
    pub fn precedence(&self) -> u8 {
        match self {
            Self::Math(Math::Pow) => 100,
            Self::Math(Math::Multiply)
            | Self::Math(Math::Divide)
            | Self::Math(Math::Modulo)
            | Self::Math(Math::FloorDivision) => 95,
            Self::Math(Math::Plus) | Self::Math(Math::Minus) => 90,
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
            | Self::Math(Math::Concat) => 80,
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

impl Display for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operator::Assignment(Assignment::Assign) => write!(f, "="),
            Operator::Assignment(Assignment::PlusAssign) => write!(f, "+="),
            Operator::Assignment(Assignment::ConcatAssign) => write!(f, "++="),
            Operator::Assignment(Assignment::MinusAssign) => write!(f, "-="),
            Operator::Assignment(Assignment::MultiplyAssign) => write!(f, "*="),
            Operator::Assignment(Assignment::DivideAssign) => write!(f, "/="),
            Operator::Comparison(Comparison::Equal) => write!(f, "=="),
            Operator::Comparison(Comparison::NotEqual) => write!(f, "!="),
            Operator::Comparison(Comparison::LessThan) => write!(f, "<"),
            Operator::Comparison(Comparison::GreaterThan) => write!(f, ">"),
            Operator::Comparison(Comparison::RegexMatch) => write!(f, "=~ or like"),
            Operator::Comparison(Comparison::NotRegexMatch) => write!(f, "!~ or not-like"),
            Operator::Comparison(Comparison::LessThanOrEqual) => write!(f, "<="),
            Operator::Comparison(Comparison::GreaterThanOrEqual) => write!(f, ">="),
            Operator::Comparison(Comparison::StartsWith) => write!(f, "starts-with"),
            Operator::Comparison(Comparison::EndsWith) => write!(f, "ends-with"),
            Operator::Comparison(Comparison::In) => write!(f, "in"),
            Operator::Comparison(Comparison::NotIn) => write!(f, "not-in"),
            Operator::Math(Math::Plus) => write!(f, "+"),
            Operator::Math(Math::Concat) => write!(f, "++"),
            Operator::Math(Math::Minus) => write!(f, "-"),
            Operator::Math(Math::Multiply) => write!(f, "*"),
            Operator::Math(Math::Divide) => write!(f, "/"),
            Operator::Math(Math::Modulo) => write!(f, "mod"),
            Operator::Math(Math::FloorDivision) => write!(f, "//"),
            Operator::Math(Math::Pow) => write!(f, "**"),
            Operator::Boolean(Boolean::And) => write!(f, "and"),
            Operator::Boolean(Boolean::Or) => write!(f, "or"),
            Operator::Boolean(Boolean::Xor) => write!(f, "xor"),
            Operator::Bits(Bits::BitOr) => write!(f, "bit-or"),
            Operator::Bits(Bits::BitXor) => write!(f, "bit-xor"),
            Operator::Bits(Bits::BitAnd) => write!(f, "bit-and"),
            Operator::Bits(Bits::ShiftLeft) => write!(f, "bit-shl"),
            Operator::Bits(Bits::ShiftRight) => write!(f, "bit-shr"),
        }
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

impl Display for RangeOperator {
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
        } => Ok(operator.clone()),
        Expression { span, expr, .. } => Err(ShellError::UnknownOperator {
            op_token: format!("{expr:?}"),
            span: *span,
        }),
    }
}
