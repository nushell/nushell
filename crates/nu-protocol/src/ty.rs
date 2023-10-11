use serde::{Deserialize, Serialize};
#[cfg(test)]
use strum_macros::EnumIter;

use std::fmt::Display;

use crate::SyntaxShape;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[cfg_attr(test, derive(EnumIter))]
pub enum Type {
    Any,
    Binary,
    Block,
    Bool,
    CellPath,
    Closure,
    Custom(String),
    Date,
    Duration,
    Error,
    Filesize,
    Float,
    Int,
    List(Box<Type>),
    ListStream,
    MatchPattern,
    #[default]
    Nothing,
    Number,
    Range,
    Record(Vec<(String, Type)>),
    Signature,
    String,
    Table(Vec<(String, Type)>),
}

impl Type {
    pub fn is_subtype(&self, other: &Type) -> bool {
        // Structural subtyping
        let is_subtype_collection = |this: &[(String, Type)], that: &[(String, Type)]| {
            if this.is_empty() || that.is_empty() {
                true
            } else if this.len() > that.len() {
                false
            } else {
                this.iter().all(|(col_x, ty_x)| {
                    if let Some((_, ty_y)) = that.iter().find(|(col_y, _)| col_x == col_y) {
                        ty_x.is_subtype(ty_y)
                    } else {
                        false
                    }
                })
            }
        };

        match (self, other) {
            (t, u) if t == u => true,
            (Type::Float, Type::Number) => true,
            (Type::Int, Type::Number) => true,
            (_, Type::Any) => true,
            (Type::List(t), Type::List(u)) if t.is_subtype(u) => true, // List is covariant
            (Type::Record(this), Type::Record(that)) | (Type::Table(this), Type::Table(that)) => {
                is_subtype_collection(this, that)
            }
            _ => false,
        }
    }

    pub fn is_numeric(&self) -> bool {
        matches!(self, Type::Int | Type::Float | Type::Number)
    }

    pub fn is_list(&self) -> bool {
        matches!(self, Type::List(_))
    }

    /// Does this type represent a data structure containing values that can be addressed using 'cell paths'?
    pub fn accepts_cell_paths(&self) -> bool {
        matches!(self, Type::List(_) | Type::Record(_) | Type::Table(_))
    }

    pub fn to_shape(&self) -> SyntaxShape {
        let mk_shape = |tys: &[(String, Type)]| {
            tys.iter()
                .map(|(key, val)| (key.clone(), val.to_shape()))
                .collect()
        };

        match self {
            Type::Int => SyntaxShape::Int,
            Type::Float => SyntaxShape::Float,
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
            Type::Record(entries) => SyntaxShape::Record(mk_shape(entries)),
            Type::Table(columns) => SyntaxShape::Table(mk_shape(columns)),
            Type::ListStream => SyntaxShape::List(Box::new(SyntaxShape::Any)),
            Type::Any => SyntaxShape::Any,
            Type::Error => SyntaxShape::Any,
            Type::Binary => SyntaxShape::Binary,
            Type::Custom(_) => SyntaxShape::Any,
            Type::Signature => SyntaxShape::Signature,
            Type::MatchPattern => SyntaxShape::MatchPattern,
        }
    }

    /// Get a string representation, without inner type specification of lists,
    /// tables and records (get `list` instead of `list<any>`
    pub fn get_non_specified_string(&self) -> String {
        match self {
            Type::Block => String::from("block"),
            Type::Closure => String::from("closure"),
            Type::Bool => String::from("bool"),
            Type::CellPath => String::from("cell-path"),
            Type::Date => String::from("date"),
            Type::Duration => String::from("duration"),
            Type::Filesize => String::from("filesize"),
            Type::Float => String::from("float"),
            Type::Int => String::from("int"),
            Type::Range => String::from("range"),
            Type::Record(_) => String::from("record"),
            Type::Table(_) => String::from("table"),
            Type::List(_) => String::from("list"),
            Type::MatchPattern => String::from("match-pattern"),
            Type::Nothing => String::from("nothing"),
            Type::Number => String::from("number"),
            Type::String => String::from("string"),
            Type::ListStream => String::from("list-stream"),
            Type::Any => String::from("any"),
            Type::Error => String::from("error"),
            Type::Binary => String::from("binary"),
            Type::Custom(_) => String::from("custom"),
            Type::Signature => String::from("signature"),
        }
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Block => write!(f, "block"),
            Type::Closure => write!(f, "closure"),
            Type::Bool => write!(f, "bool"),
            Type::CellPath => write!(f, "cell-path"),
            Type::Date => write!(f, "date"),
            Type::Duration => write!(f, "duration"),
            Type::Filesize => write!(f, "filesize"),
            Type::Float => write!(f, "float"),
            Type::Int => write!(f, "int"),
            Type::Range => write!(f, "range"),
            Type::Record(fields) => {
                if fields.is_empty() {
                    write!(f, "record")
                } else {
                    write!(
                        f,
                        "record<{}>",
                        fields
                            .iter()
                            .map(|(x, y)| format!("{x}: {y}"))
                            .collect::<Vec<String>>()
                            .join(", "),
                    )
                }
            }
            Type::Table(columns) => {
                if columns.is_empty() {
                    write!(f, "table")
                } else {
                    write!(
                        f,
                        "table<{}>",
                        columns
                            .iter()
                            .map(|(x, y)| format!("{x}: {y}"))
                            .collect::<Vec<String>>()
                            .join(", ")
                    )
                }
            }
            Type::List(l) => write!(f, "list<{l}>"),
            Type::Nothing => write!(f, "nothing"),
            Type::Number => write!(f, "number"),
            Type::String => write!(f, "string"),
            Type::ListStream => write!(f, "list-stream"),
            Type::Any => write!(f, "any"),
            Type::Error => write!(f, "error"),
            Type::Binary => write!(f, "binary"),
            Type::Custom(custom) => write!(f, "{custom}"),
            Type::Signature => write!(f, "signature"),
            Type::MatchPattern => write!(f, "match-pattern"),
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
