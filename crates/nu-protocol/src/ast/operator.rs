use crate::Span;

use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Operator {
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessThanOrEqual,
    GreaterThanOrEqual,
    Contains,
    NotContains,
    Plus,
    Minus,
    Multiply,
    Divide,
    In,
    NotIn,
    Modulo,
    And,
    Or,
    Pow,
}

impl Display for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operator::Equal => write!(f, "=="),
            Operator::NotEqual => write!(f, "!="),
            Operator::LessThan => write!(f, "<"),
            Operator::GreaterThan => write!(f, ">"),
            Operator::Contains => write!(f, "=~"),
            Operator::NotContains => write!(f, "!~"),
            Operator::Plus => write!(f, "+"),
            Operator::Minus => write!(f, "-"),
            Operator::Multiply => write!(f, "*"),
            Operator::Divide => write!(f, "/"),
            Operator::In => write!(f, "in"),
            Operator::NotIn => write!(f, "not-in"),
            Operator::Modulo => write!(f, "mod"),
            Operator::And => write!(f, "&&"),
            Operator::Or => write!(f, "||"),
            Operator::Pow => write!(f, "**"),
            Operator::LessThanOrEqual => write!(f, "<="),
            Operator::GreaterThanOrEqual => write!(f, ">="),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum RangeInclusion {
    Inclusive,
    RightExclusive,
}

#[derive(Debug, Copy, Clone)]
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
