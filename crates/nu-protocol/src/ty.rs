use serde::{Deserialize, Serialize};

use std::fmt::Display;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    Table,
    ValueStream,
    Unknown,
    Error,
    Binary,
    Custom,
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
                    .map(|(x, y)| format!("{}: {}", x, y.to_string()))
                    .collect::<Vec<String>>()
                    .join(", "),
            ),
            Type::Table => write!(f, "table"),
            Type::List(l) => write!(f, "list<{}>", l),
            Type::Nothing => write!(f, "nothing"),
            Type::Number => write!(f, "number"),
            Type::String => write!(f, "string"),
            Type::ValueStream => write!(f, "value stream"),
            Type::Unknown => write!(f, "unknown"),
            Type::Error => write!(f, "error"),
            Type::Binary => write!(f, "binary"),
            Type::Custom => write!(f, "custom"),
        }
    }
}
