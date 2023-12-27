use super::Expression;
use crate::Span;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

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
        reverse: bool,
    },
}

impl PathMember {
    /**
    Generates a new `Pathmember::Int`, given a `i64`, `Span` and an `optional`.
    */
    pub fn new_int(int: i64, span: Span, optional: bool) -> PathMember {
        if int < 0 {
            return PathMember::Int {
                val: (int.abs() as usize),
                span,
                optional,
                reverse: true,
            };
        }
        PathMember::Int {
            val: int as usize,
            span,
            optional,
            reverse: false,
        }
    }
    /**
    Generates a new `Pathmember::String` given a `String`, `Span` an an `optional`.
    */
    pub fn new_string(string: String, span: Span, optional: bool) -> PathMember {
        PathMember::String {
            val: string,
            span,
            optional,
        }
    }
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
                    reverse: l_rev,
                    ..
                },
                Self::Int {
                    val: r_val,
                    optional: r_opt,
                    reverse: r_rev,
                    ..
                },
            ) => l_val == r_val && l_opt == r_opt && l_rev == r_rev,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct CellPath {
    pub members: Vec<PathMember>,
}

impl CellPath {
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

impl Display for CellPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (idx, elem) in self.members.iter().enumerate() {
            if idx > 0 {
                write!(f, ".")?;
            }
            match elem {
                PathMember::Int { val, .. } => write!(f, "{val}")?,
                PathMember::String { val, .. } => write!(f, "{val}")?,
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FullCellPath {
    pub head: Expression,
    pub tail: Vec<PathMember>,
}
