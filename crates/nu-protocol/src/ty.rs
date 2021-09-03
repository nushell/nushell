use std::fmt::Display;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    Int,
    Float,
    Bool,
    String,
    Block,
    ColumnPath,
    Duration,
    FilePath,
    Filesize,
    List(Box<Type>),
    Number,
    Nothing,
    Table,
    Unknown,
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Block => write!(f, "block"),
            Type::Bool => write!(f, "bool"),
            Type::ColumnPath => write!(f, "column path"),
            Type::Duration => write!(f, "duration"),
            Type::FilePath => write!(f, "filepath"),
            Type::Filesize => write!(f, "filesize"),
            Type::Float => write!(f, "float"),
            Type::Int => write!(f, "int"),
            Type::List(l) => write!(f, "list<{}>", l),
            Type::Nothing => write!(f, "nothing"),
            Type::Number => write!(f, "number"),
            Type::String => write!(f, "string"),
            Type::Table => write!(f, "table"),
            Type::Unknown => write!(f, "unknown"),
        }
    }
}
