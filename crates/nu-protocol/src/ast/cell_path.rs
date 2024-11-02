use super::Expression;
use crate::Span;
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, fmt::Display};

/// One level of access of a [`CellPath`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PathMember {
    /// Accessing a member by string (i.e. columns of a table or [`Record`](crate::Record))
    String {
        val: String,
        span: Span,
        /// If marked as optional don't throw an error if not found but perform default handling
        /// (e.g. return `Value::Nothing`)
        optional: bool,
    },
    /// Accessing a member by index (i.e. row of a table or item in a list)
    Int {
        val: usize,
        span: Span,
        /// If marked as optional don't throw an error if not found but perform default handling
        /// (e.g. return `Value::Nothing`)
        optional: bool,
    },
}

impl PathMember {
    pub fn int(val: usize, optional: bool, span: Span) -> Self {
        PathMember::Int {
            val,
            span,
            optional,
        }
    }

    pub fn string(val: String, optional: bool, span: Span) -> Self {
        PathMember::String {
            val,
            span,
            optional,
        }
    }

    pub fn test_int(val: usize, optional: bool) -> Self {
        PathMember::Int {
            val,
            optional,
            span: Span::test_data(),
        }
    }

    pub fn test_string(val: String, optional: bool) -> Self {
        PathMember::String {
            val,
            optional,
            span: Span::test_data(),
        }
    }

    pub fn make_optional(&mut self) {
        match self {
            PathMember::String {
                ref mut optional, ..
            } => *optional = true,
            PathMember::Int {
                ref mut optional, ..
            } => *optional = true,
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

impl PartialOrd for PathMember {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (
                PathMember::String {
                    val: l_val,
                    optional: l_opt,
                    ..
                },
                PathMember::String {
                    val: r_val,
                    optional: r_opt,
                    ..
                },
            ) => {
                let val_ord = Some(l_val.cmp(r_val));

                if let Some(Ordering::Equal) = val_ord {
                    Some(l_opt.cmp(r_opt))
                } else {
                    val_ord
                }
            }
            (
                PathMember::Int {
                    val: l_val,
                    optional: l_opt,
                    ..
                },
                PathMember::Int {
                    val: r_val,
                    optional: r_opt,
                    ..
                },
            ) => {
                let val_ord = Some(l_val.cmp(r_val));

                if let Some(Ordering::Equal) = val_ord {
                    Some(l_opt.cmp(r_opt))
                } else {
                    val_ord
                }
            }
            (PathMember::Int { .. }, PathMember::String { .. }) => Some(Ordering::Greater),
            (PathMember::String { .. }, PathMember::Int { .. }) => Some(Ordering::Less),
        }
    }
}

/// Represents the potentially nested access to fields/cells of a container type
///
/// In our current implementation for table access the order of row/column is commutative.
/// This limits the number of possible rows to select in one [`CellPath`] to 1 as it could
/// otherwise be ambiguous
///
/// ```nushell
/// col1.0
/// 0.col1
/// col2
/// 42
/// ```
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct CellPath {
    pub members: Vec<PathMember>,
}

impl CellPath {
    pub fn make_optional(&mut self) {
        for member in &mut self.members {
            member.make_optional();
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

#[cfg(test)]
mod test {
    use super::*;
    use std::cmp::Ordering::Greater;

    #[test]
    fn path_member_partial_ord() {
        assert_eq!(
            Some(Greater),
            PathMember::test_int(5, true).partial_cmp(&PathMember::test_string("e".into(), true))
        );

        assert_eq!(
            Some(Greater),
            PathMember::test_int(5, true).partial_cmp(&PathMember::test_int(5, false))
        );

        assert_eq!(
            Some(Greater),
            PathMember::test_int(6, true).partial_cmp(&PathMember::test_int(5, true))
        );

        assert_eq!(
            Some(Greater),
            PathMember::test_string("e".into(), true)
                .partial_cmp(&PathMember::test_string("e".into(), false))
        );

        assert_eq!(
            Some(Greater),
            PathMember::test_string("f".into(), true)
                .partial_cmp(&PathMember::test_string("e".into(), true))
        );
    }
}
