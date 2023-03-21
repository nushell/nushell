use serde::{Deserialize, Serialize};

use crate::{Span, VarId};

use super::Expression;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchPattern {
    pub pattern: Pattern,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Pattern {
    Record(Vec<(MatchPattern, MatchPattern)>),
    Value(Expression),
    Variable(VarId),
    Garbage,
}
