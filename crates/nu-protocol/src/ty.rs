use std::fmt::Display;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    Int,
    Float,
    Range,
    Bool,
    String,
    Block,
    CellPath,
    Duration,
    FilePath,
    Filesize,
    List(Box<Type>),
    Number,
    Nothing,
    Record(Vec<String>, Vec<Type>),
    Table,
    ValueStream,
    Unknown,
    Error,
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Block => write!(f, "block"),
            Type::Bool => write!(f, "bool"),
            Type::CellPath => write!(f, "cell path"),
            Type::Duration => write!(f, "duration"),
            Type::FilePath => write!(f, "filepath"),
            Type::Filesize => write!(f, "filesize"),
            Type::Float => write!(f, "float"),
            Type::Int => write!(f, "int"),
            Type::Range => write!(f, "range"),
            Type::Record(cols, vals) => write!(f, "record<{}, {:?}>", cols.join(", "), vals),
            Type::Table => write!(f, "table"),
            Type::List(l) => write!(f, "list<{}>", l),
            Type::Nothing => write!(f, "nothing"),
            Type::Number => write!(f, "number"),
            Type::String => write!(f, "string"),
            Type::ValueStream => write!(f, "value stream"),
            Type::Unknown => write!(f, "unknown"),
            Type::Error => write!(f, "error"),
        }
    }
}
