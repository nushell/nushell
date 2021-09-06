use super::Expression;
use crate::Span;

#[derive(Debug, Clone)]
pub enum PathMember {
    String { val: String, span: Span },
    Int { val: usize, span: Span },
}

#[derive(Debug, Clone)]
pub struct CellPath {
    pub members: Vec<PathMember>,
}

#[derive(Debug, Clone)]
pub struct FullCellPath {
    pub head: Expression,
    pub tail: Vec<PathMember>,
}
