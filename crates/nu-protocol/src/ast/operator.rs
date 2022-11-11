use crate::Span;

use serde::{Deserialize, Serialize};
use std::fmt::Display;

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
    Append,
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

impl Display for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operator::Assignment(Assignment::Assign) => write!(f, "="),
            Operator::Assignment(Assignment::PlusAssign) => write!(f, "+="),
            Operator::Assignment(Assignment::MinusAssign) => write!(f, "-="),
            Operator::Assignment(Assignment::MultiplyAssign) => write!(f, "*="),
            Operator::Assignment(Assignment::DivideAssign) => write!(f, "/="),
            Operator::Comparison(Comparison::Equal) => write!(f, "=="),
            Operator::Comparison(Comparison::NotEqual) => write!(f, "!="),
            Operator::Comparison(Comparison::LessThan) => write!(f, "<"),
            Operator::Comparison(Comparison::GreaterThan) => write!(f, ">"),
            Operator::Comparison(Comparison::RegexMatch) => write!(f, "=~"),
            Operator::Comparison(Comparison::NotRegexMatch) => write!(f, "!~"),
            Operator::Comparison(Comparison::LessThanOrEqual) => write!(f, "<="),
            Operator::Comparison(Comparison::GreaterThanOrEqual) => write!(f, ">="),
            Operator::Comparison(Comparison::StartsWith) => write!(f, "starts-with"),
            Operator::Comparison(Comparison::EndsWith) => write!(f, "ends-with"),
            Operator::Comparison(Comparison::In) => write!(f, "in"),
            Operator::Comparison(Comparison::NotIn) => write!(f, "not-in"),
            Operator::Math(Math::Plus) => write!(f, "+"),
            Operator::Math(Math::Append) => write!(f, "++"),
            Operator::Math(Math::Minus) => write!(f, "-"),
            Operator::Math(Math::Multiply) => write!(f, "*"),
            Operator::Math(Math::Divide) => write!(f, "/"),
            Operator::Math(Math::Modulo) => write!(f, "mod"),
            Operator::Math(Math::FloorDivision) => write!(f, "fdiv"),
            Operator::Math(Math::Pow) => write!(f, "**"),
            Operator::Boolean(Boolean::And) => write!(f, "&&"),
            Operator::Boolean(Boolean::Or) => write!(f, "||"),
            Operator::Bits(Bits::BitOr) => write!(f, "bit-or"),
            Operator::Bits(Bits::BitXor) => write!(f, "bit-xor"),
            Operator::Bits(Bits::BitAnd) => write!(f, "bit-and"),
            Operator::Bits(Bits::ShiftLeft) => write!(f, "bit-shl"),
            Operator::Bits(Bits::ShiftRight) => write!(f, "bit-shr"),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Serialize, Deserialize)]
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
