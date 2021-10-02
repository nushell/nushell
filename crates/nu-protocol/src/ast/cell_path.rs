use super::Expression;
use crate::Span;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PathMember {
    String { val: String, span: Span },
    Int { val: usize, span: Span },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellPath {
    pub members: Vec<PathMember>,
}

#[derive(Debug, Clone)]
pub struct FullCellPath {
    pub head: Expression,
    pub tail: Vec<PathMember>,
}
