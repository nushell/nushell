use super::Expression;
use crate::Span;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Keyword {
    pub keyword: Box<[u8]>,
    pub span: Span,
    pub expr: Expression,
}
