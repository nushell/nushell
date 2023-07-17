use super::Expression;
use crate::Span;
use serde::{Deserialize, Serialize};
use std::fmt::Write;

#[derive(Debug, Clone, PartialOrd, Serialize, Deserialize)]
pub enum PathMember {
    String {
        val: String,
        span: Span,
        optional: bool,
    },
    Int {
        val: usize,
        span: Span,
        optional: bool,
    },
}

impl PartialEq for PathMember {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::String {
                    val: l_val,
                    optional: l_opt,
                    ..
                },
                Self::String {
                    val: r_val,
                    optional: r_opt,
                    ..
                },
            ) => l_val == r_val && l_opt == r_opt,
            (
                Self::Int {
                    val: l_val,
                    optional: l_opt,
                    ..
                },
                Self::Int {
                    val: r_val,
                    optional: r_opt,
                    ..
                },
            ) => l_val == r_val && l_opt == r_opt,
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
        let mut output = String::from("$.");

        for (idx, elem) in self.members.iter().enumerate() {
            if idx > 0 {
                output.push('.');
            }
            match elem {
                PathMember::Int { val, .. } => {
                    let _ = write!(output, "{val}");
                }
                PathMember::String { val, .. } => output.push_str(val),
            }
        }

        output
    }

    pub fn make_optional(&mut self) {
        for member in &mut self.members {
            match member {
                PathMember::String {
                    ref mut optional, ..
                } => *optional = true,
                PathMember::Int {
                    ref mut optional, ..
                } => *optional = true,
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FullCellPath {
    pub head: Expression,
    pub tail: Vec<PathMember>,
}
