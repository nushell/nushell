use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::{DeclId, Type};

/// The syntactic shapes that values must match to be passed into a command. You can think of this as the type-checking that occurs when you call a function.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyntaxShape {
    /// Any syntactic form is allowed
    Any,

    /// A binary literal
    Binary,

    /// A block is allowed, eg `{start this thing}`
    Block,

    /// A boolean value, eg `true` or `false`
    Boolean,

    /// A dotted path to navigate the table
    CellPath,

    /// A closure is allowed, eg `{|| start this thing}`
    Closure(Option<Vec<SyntaxShape>>),

    /// A custom shape with custom completion logic
    Custom(Box<SyntaxShape>, DeclId),

    /// A datetime value, eg `2022-02-02` or `2019-10-12T07:20:50.52+00:00`
    DateTime,

    /// A directory is allowed
    Directory,

    /// A duration value is allowed, eg `19day`
    Duration,

    /// An error value
    Error,

    /// A general expression, eg `1 + 2` or `foo --bar`
    Expression,

    /// A filepath is allowed
    Filepath,

    /// A filesize value is allowed, eg `10kb`
    Filesize,

    /// A dotted path to navigate the table (including variable)
    FullCellPath,

    /// A glob pattern is allowed, eg `foo*`
    GlobPattern,

    /// Only an integer value is allowed
    Int,

    /// A module path pattern used for imports
    ImportPattern,

    /// A specific match to a word or symbol
    Keyword(Vec<u8>, Box<SyntaxShape>),

    /// A list is allowed, eg `[first second]`
    List(Box<SyntaxShape>),

    /// A general math expression, eg `1 + 2`
    MathExpression,

    /// Nothing
    Nothing,

    /// Only a numeric (integer or decimal) value is allowed
    Number,

    /// One of a list of possible items, checked in order
    OneOf(Vec<SyntaxShape>),

    /// An operator, eg `+`
    Operator,

    /// A range is allowed (eg, `1..3`)
    Range,

    /// A record value, eg `{x: 1, y: 2}`
    Record,

    /// A math expression which expands shorthand forms on the lefthand side, eg `foo > 1`
    /// The shorthand allows us to more easily reach columns inside of the row being passed in
    RowCondition,

    /// A signature for a definition, `[x:int, --foo]`
    Signature,

    /// Strings and string-like bare words are allowed
    String,

    /// A table is allowed, eg `[[first, second]; [1, 2]]`
    Table,

    /// A variable name, eg `$foo`
    Variable,

    /// A variable with optional type, `x` or `x: int`
    VarWithOptType,
}

impl SyntaxShape {
    pub fn to_type(&self) -> Type {
        match self {
            SyntaxShape::Any => Type::Any,
            SyntaxShape::Block => Type::Block,
            SyntaxShape::Closure(_) => Type::Closure,
            SyntaxShape::Binary => Type::Binary,
            SyntaxShape::CellPath => Type::Any,
            SyntaxShape::Custom(custom, _) => custom.to_type(),
            SyntaxShape::DateTime => Type::Date,
            SyntaxShape::Duration => Type::Duration,
            SyntaxShape::Expression => Type::Any,
            SyntaxShape::Filepath => Type::String,
            SyntaxShape::Directory => Type::String,
            SyntaxShape::Filesize => Type::Filesize,
            SyntaxShape::FullCellPath => Type::Any,
            SyntaxShape::GlobPattern => Type::String,
            SyntaxShape::Error => Type::Error,
            SyntaxShape::ImportPattern => Type::Any,
            SyntaxShape::Int => Type::Int,
            SyntaxShape::List(x) => {
                let contents = x.to_type();
                Type::List(Box::new(contents))
            }
            SyntaxShape::Keyword(_, expr) => expr.to_type(),
            SyntaxShape::MathExpression => Type::Any,
            SyntaxShape::Nothing => Type::Any,
            SyntaxShape::Number => Type::Number,
            SyntaxShape::OneOf(_) => Type::Any,
            SyntaxShape::Operator => Type::Any,
            SyntaxShape::Range => Type::Any,
            SyntaxShape::Record => Type::Record(vec![]), // FIXME: What role should fields play in the Record type?
            SyntaxShape::RowCondition => Type::Bool,
            SyntaxShape::Boolean => Type::Bool,
            SyntaxShape::Signature => Type::Signature,
            SyntaxShape::String => Type::String,
            SyntaxShape::Table => Type::List(Box::new(Type::Any)), // FIXME:  What role should columns play in the Table type?
            SyntaxShape::VarWithOptType => Type::Any,
            SyntaxShape::Variable => Type::Any,
        }
    }
}

impl Display for SyntaxShape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyntaxShape::Keyword(kw, shape) => {
                write!(f, "\"{}\" {}", String::from_utf8_lossy(kw), shape)
            }
            SyntaxShape::Any => write!(f, "any"),
            SyntaxShape::String => write!(f, "string"),
            SyntaxShape::CellPath => write!(f, "cellpath"),
            SyntaxShape::FullCellPath => write!(f, "cellpath"),
            SyntaxShape::Number => write!(f, "number"),
            SyntaxShape::Range => write!(f, "range"),
            SyntaxShape::Int => write!(f, "int"),
            SyntaxShape::Filepath => write!(f, "path"),
            SyntaxShape::Directory => write!(f, "directory"),
            SyntaxShape::GlobPattern => write!(f, "glob"),
            SyntaxShape::ImportPattern => write!(f, "import"),
            SyntaxShape::Block => write!(f, "block"),
            SyntaxShape::Closure(args) => {
                if let Some(args) = args {
                    let arg_vec: Vec<_> = args.iter().map(|x| x.to_string()).collect();
                    let arg_string = arg_vec.join(", ");
                    write!(f, "closure({})", arg_string)
                } else {
                    write!(f, "closure()")
                }
            }
            SyntaxShape::Binary => write!(f, "binary"),
            SyntaxShape::Table => write!(f, "table"),
            SyntaxShape::List(x) => write!(f, "list<{}>", x),
            SyntaxShape::Record => write!(f, "record"),
            SyntaxShape::Filesize => write!(f, "filesize"),
            SyntaxShape::Duration => write!(f, "duration"),
            SyntaxShape::DateTime => write!(f, "datetime"),
            SyntaxShape::Operator => write!(f, "operator"),
            SyntaxShape::RowCondition => write!(f, "condition"),
            SyntaxShape::MathExpression => write!(f, "variable"),
            SyntaxShape::Variable => write!(f, "var"),
            SyntaxShape::VarWithOptType => write!(f, "vardecl"),
            SyntaxShape::Signature => write!(f, "signature"),
            SyntaxShape::Expression => write!(f, "expression"),
            SyntaxShape::Boolean => write!(f, "bool"),
            SyntaxShape::Error => write!(f, "error"),
            SyntaxShape::Custom(x, _) => write!(f, "custom<{}>", x),
            SyntaxShape::OneOf(list) => {
                let arg_vec: Vec<_> = list.iter().map(|x| x.to_string()).collect();
                let arg_string = arg_vec.join(", ");
                write!(f, "one_of({})", arg_string)
            }
            SyntaxShape::Nothing => write!(f, "nothing"),
        }
    }
}
