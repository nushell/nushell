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

impl CellPath {
    pub fn into_string(&self) -> String {
        let mut output = String::new();

        for (idx, elem) in self.members.iter().enumerate() {
            if idx > 0 {
                output.push('.');
            }
            match elem {
                PathMember::Int { val, .. } => output.push_str(&format!("{}", val)),
                PathMember::String { val, .. } => output.push_str(val),
            }
        }

        output
    }
}

#[derive(Debug, Clone)]
pub struct FullCellPath {
    pub head: Expression,
    pub tail: Vec<PathMember>,
}
