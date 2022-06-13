use serde::{Deserialize, Serialize};

use std::fmt::Display;

use crate::SyntaxShape;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum Type {
    Int,
    Float,
    Range,
    Bool,
    String,
    Block,
    CellPath,
    Duration,
    Date,
    Filesize,
    List(Box<Type>),
    Number,
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
    pub fn to_shape(&self) -> SyntaxShape {
        match self {
            Type::Int => SyntaxShape::Int,
            Type::Float => SyntaxShape::Number,
            Type::Range => SyntaxShape::Range,
            Type::Bool => SyntaxShape::Boolean,
            Type::String => SyntaxShape::String,
            Type::Block => SyntaxShape::Block(None), // FIXME needs more accuracy
            Type::CellPath => SyntaxShape::CellPath,
            Type::Duration => SyntaxShape::Duration,
            Type::Date => SyntaxShape::DateTime,
            Type::Filesize => SyntaxShape::Filesize,
            Type::List(x) => SyntaxShape::List(Box::new(x.to_shape())),
            Type::Number => SyntaxShape::Number,
            Type::Nothing => SyntaxShape::Any,
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
