use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;

use std::fmt::Display;

use crate::SyntaxShape;

#[derive(Clone, Debug, Default, EnumIter, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum Type {
    Int,
    Float,
    Range,
    Bool,
    String,
    Block,
    Closure,
    CellPath,
    Duration,
    Date,
    Filesize,
    List(Box<Type>),
    Number,
    #[default]
    Nothing,
    Record(Vec<(String, Type)>),
    Table(Vec<(String, Type)>),
    ListStream,
    Any,
    Error,
    Binary,
    Custom(String),
    Signature,
}

impl Type {
    pub fn is_subtype(&self, other: &Type) -> bool {
        match (self, other) {
            (t, u) if t == u => true,
            (Type::Float, Type::Number) => true,
            (Type::Int, Type::Number) => true,
            (_, Type::Any) => true,
            (Type::List(t), Type::List(u)) if t.is_subtype(u) => true, // List is covariant

            // TODO: Currently Record types specify their field types. If we are
            // going to continue to do that, then it might make sense to define
            // a "structural subtyping" whereby r1 is a subtype of r2 is the
            // fields of r1 are a "subset" of the fields of r2 (names are a
            // subset and agree on types). However, if we do that, then we need
            // a way to specify the supertype of all Records. For now, we define
            // any Record to be a subtype of any other Record. This allows
            // Record(vec![]) to be used as an ad-hoc supertype of all Records
            // in command signatures. This comment applies to Tables also, with
            // "columns" in place of "fields".
            (Type::Record(_), Type::Record(_)) => true,
            (Type::Table(_), Type::Table(_)) => true,
            _ => false,
        }
    }

    pub fn is_numeric(&self) -> bool {
        matches!(self, Type::Int | Type::Float | Type::Number)
    }

    /// Does this type represent a data structure containing values that can be addressed using 'cell paths'?
    pub fn accepts_cell_paths(&self) -> bool {
        matches!(self, Type::List(_) | Type::Record(_) | Type::Table(_))
    }

    pub fn to_shape(&self) -> SyntaxShape {
        match self {
            Type::Int => SyntaxShape::Int,
            Type::Float => SyntaxShape::Number,
            Type::Range => SyntaxShape::Range,
            Type::Bool => SyntaxShape::Boolean,
            Type::String => SyntaxShape::String,
            Type::Block => SyntaxShape::Block, // FIXME needs more accuracy
            Type::Closure => SyntaxShape::Closure(None), // FIXME needs more accuracy
            Type::CellPath => SyntaxShape::CellPath,
            Type::Duration => SyntaxShape::Duration,
            Type::Date => SyntaxShape::DateTime,
            Type::Filesize => SyntaxShape::Filesize,
            Type::List(x) => SyntaxShape::List(Box::new(x.to_shape())),
            Type::Number => SyntaxShape::Number,
            Type::Nothing => SyntaxShape::Nothing,
            Type::Record(_) => SyntaxShape::Record,
            Type::Table(_) => SyntaxShape::Table,
            Type::ListStream => SyntaxShape::List(Box::new(SyntaxShape::Any)),
            Type::Any => SyntaxShape::Any,
            Type::Error => SyntaxShape::Any,
            Type::Binary => SyntaxShape::Binary,
            Type::Custom(_) => SyntaxShape::Any,
            Type::Signature => SyntaxShape::Signature,
        }
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Block => write!(f, "block"),
            Type::Closure => write!(f, "closure"),
            Type::Bool => write!(f, "bool"),
            Type::CellPath => write!(f, "cell path"),
            Type::Date => write!(f, "date"),
            Type::Duration => write!(f, "duration"),
            Type::Filesize => write!(f, "filesize"),
            Type::Float => write!(f, "float"),
            Type::Int => write!(f, "int"),
            Type::Range => write!(f, "range"),
            Type::Record(fields) => write!(
                f,
                "record<{}>",
                fields
                    .iter()
                    .map(|(x, y)| format!("{}: {}", x, y))
                    .collect::<Vec<String>>()
                    .join(", "),
            ),
            Type::Table(columns) => write!(
                f,
                "table<{}>",
                columns
                    .iter()
                    .map(|(x, y)| format!("{}: {}", x, y))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Type::List(l) => write!(f, "list<{}>", l),
            Type::Nothing => write!(f, "nothing"),
            Type::Number => write!(f, "number"),
            Type::String => write!(f, "string"),
            Type::ListStream => write!(f, "list stream"),
            Type::Any => write!(f, "any"),
            Type::Error => write!(f, "error"),
            Type::Binary => write!(f, "binary"),
            Type::Custom(custom) => write!(f, "{}", custom),
            Type::Signature => write!(f, "signature"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Type;
    use strum::IntoEnumIterator;

    mod subtype_relation {
        use super::*;

        #[test]
        fn test_reflexivity() {
            for ty in Type::iter() {
                assert!(ty.is_subtype(&ty));
            }
        }

        #[test]
        fn test_any_is_top_type() {
            for ty in Type::iter() {
                assert!(ty.is_subtype(&Type::Any));
            }
        }

        #[test]
        fn test_number_supertype() {
            assert!(Type::Int.is_subtype(&Type::Number));
            assert!(Type::Float.is_subtype(&Type::Number));
        }

        #[test]
        fn test_list_covariance() {
            for ty1 in Type::iter() {
                for ty2 in Type::iter() {
                    let list_ty1 = Type::List(Box::new(ty1.clone()));
                    let list_ty2 = Type::List(Box::new(ty2.clone()));
                    assert_eq!(list_ty1.is_subtype(&list_ty2), ty1.is_subtype(&ty2));
                }
            }
        }
    }
}
