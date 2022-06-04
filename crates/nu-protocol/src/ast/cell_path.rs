use super::Expression;
use crate::Span;
use serde::{Deserialize, Serialize};
use std::fmt::Write;

#[derive(Debug, Clone, PartialOrd, Serialize, Deserialize)]
pub enum PathMember {
    String { val: String, span: Span },
    Int { val: usize, span: Span },
}

impl PartialEq for PathMember {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::String { val: l_val, .. }, Self::String { val: r_val, .. }) => l_val == r_val,
            (Self::Int { val: l_val, .. }, Self::Int { val: r_val, .. }) => l_val == r_val,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
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
                PathMember::Int { val, .. } => {
                    let _ = write!(output, "{}", val);
                }
                PathMember::String { val, .. } => output.push_str(val),
            }
        }

        output
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FullCellPath {
    pub head: Expression,
    pub tail: Vec<PathMember>,
}
