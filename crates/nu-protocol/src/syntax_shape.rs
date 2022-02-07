<<<<<<< HEAD
use nu_source::{DbgDocBldr, DebugDocBuilder, PrettyDebug};
use serde::{Deserialize, Serialize};

/// The syntactic shapes that values must match to be passed into a command. You can think of this as the type-checking that occurs when you call a function.
#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SyntaxShape {
    /// Any syntactic form is allowed
    Any,
    /// Strings and string-like bare words are allowed
    String,
    /// A dotted path to navigate the table
    ColumnPath,
    /// A dotted path to navigate the table (including variable)
    FullColumnPath,
    /// Only a numeric (integer or decimal) value is allowed
    Number,
    /// A range is allowed (eg, `1..3`)
    Range,
    /// Only an integer value is allowed
    Int,
    /// A filepath is allowed
    FilePath,
    /// A glob pattern is allowed, eg `foo*`
    GlobPattern,
    /// A block is allowed, eg `{start this thing}`
    Block,
    /// A table is allowed, eg `[first second]`
    Table,
    /// A filesize value is allowed, eg `10kb`
    Filesize,
    /// A duration value is allowed, eg `19day`
    Duration,
    /// An operator
    Operator,
    /// A math expression which expands shorthand forms on the lefthand side, eg `foo > 1`
    /// The shorthand allows us to more easily reach columns inside of the row being passed in
    RowCondition,
    /// A general math expression, eg the `1 + 2` of `= 1 + 2`
    MathExpression,
}

impl SyntaxShape {
    pub fn syntax_shape_name(&self) -> &str {
        match self {
            SyntaxShape::Any => "any",
            SyntaxShape::String => "string",
            SyntaxShape::FullColumnPath => "column path (with variable)",
            SyntaxShape::ColumnPath => "column path",
            SyntaxShape::Number => "number",
            SyntaxShape::Range => "range",
            SyntaxShape::Int => "integer",
            SyntaxShape::FilePath => "file path",
            SyntaxShape::GlobPattern => "pattern",
            SyntaxShape::Block => "block",
            SyntaxShape::Table => "table",
            SyntaxShape::Duration => "duration",
            SyntaxShape::Filesize => "filesize",
            SyntaxShape::Operator => "operator",
            SyntaxShape::RowCondition => "condition",
            SyntaxShape::MathExpression => "math expression",
=======
use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::Type;

/// The syntactic shapes that values must match to be passed into a command. You can think of this as the type-checking that occurs when you call a function.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyntaxShape {
    /// A specific match to a word or symbol
    Keyword(Vec<u8>, Box<SyntaxShape>),

    /// Any syntactic form is allowed
    Any,

    /// Strings and string-like bare words are allowed
    String,

    /// A dotted path to navigate the table
    CellPath,

    /// A dotted path to navigate the table (including variable)
    FullCellPath,

    /// Only a numeric (integer or decimal) value is allowed
    Number,

    /// A range is allowed (eg, `1..3`)
    Range,

    /// Only an integer value is allowed
    Int,

    /// A filepath is allowed
    Filepath,

    /// A glob pattern is allowed, eg `foo*`
    GlobPattern,

    /// A module path pattern used for imports
    ImportPattern,

    /// A block is allowed, eg `{start this thing}`
    Block(Option<Vec<SyntaxShape>>),

    /// A table is allowed, eg `[[first, second]; [1, 2]]`
    Table,

    /// A table is allowed, eg `[first second]`
    List(Box<SyntaxShape>),

    /// A filesize value is allowed, eg `10kb`
    Filesize,

    /// A duration value is allowed, eg `19day`
    Duration,

    /// An operator
    Operator,

    /// A math expression which expands shorthand forms on the lefthand side, eg `foo > 1`
    /// The shorthand allows us to more easily reach columns inside of the row being passed in
    RowCondition,

    /// A general math expression, eg `1 + 2`
    MathExpression,

    /// A variable name
    Variable,

    /// A variable with optional type, `x` or `x: int`
    VarWithOptType,

    /// A signature for a definition, `[x:int, --foo]`
    Signature,

    /// A general expression, eg `1 + 2` or `foo --bar`
    Expression,

    /// A boolean value
    Boolean,

    /// A record value
    Record,

    /// A custom shape with custom completion logic
    Custom(Box<SyntaxShape>, String),
}

impl SyntaxShape {
    pub fn to_type(&self) -> Type {
        match self {
            SyntaxShape::Any => Type::Unknown,
            SyntaxShape::Block(_) => Type::Block,
            SyntaxShape::CellPath => Type::Unknown,
            SyntaxShape::Custom(custom, _) => custom.to_type(),
            SyntaxShape::Duration => Type::Duration,
            SyntaxShape::Expression => Type::Unknown,
            SyntaxShape::Filepath => Type::String,
            SyntaxShape::Filesize => Type::Filesize,
            SyntaxShape::FullCellPath => Type::Unknown,
            SyntaxShape::GlobPattern => Type::String,
            SyntaxShape::ImportPattern => Type::Unknown,
            SyntaxShape::Int => Type::Int,
            SyntaxShape::List(x) => {
                let contents = x.to_type();
                Type::List(Box::new(contents))
            }
            SyntaxShape::Keyword(_, expr) => expr.to_type(),
            SyntaxShape::MathExpression => Type::Unknown,
            SyntaxShape::Number => Type::Number,
            SyntaxShape::Operator => Type::Unknown,
            SyntaxShape::Range => Type::Unknown,
            SyntaxShape::Record => Type::Record(vec![]), // FIXME: Add actual record type
            SyntaxShape::RowCondition => Type::Bool,
            SyntaxShape::Boolean => Type::Bool,
            SyntaxShape::Signature => Type::Signature,
            SyntaxShape::String => Type::String,
            SyntaxShape::Table => Type::List(Box::new(Type::Unknown)), // FIXME: Tables should have better types
            SyntaxShape::VarWithOptType => Type::Unknown,
            SyntaxShape::Variable => Type::Unknown,
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        }
    }
}

<<<<<<< HEAD
impl PrettyDebug for SyntaxShape {
    /// Prepare SyntaxShape for pretty-printing
    fn pretty(&self) -> DebugDocBuilder {
        DbgDocBldr::kind(self.syntax_shape_name().to_string())
=======
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
            SyntaxShape::GlobPattern => write!(f, "glob"),
            SyntaxShape::ImportPattern => write!(f, "import"),
            SyntaxShape::Block(_) => write!(f, "block"),
            SyntaxShape::Table => write!(f, "table"),
            SyntaxShape::List(x) => write!(f, "list<{}>", x),
            SyntaxShape::Record => write!(f, "record"),
            SyntaxShape::Filesize => write!(f, "filesize"),
            SyntaxShape::Duration => write!(f, "duration"),
            SyntaxShape::Operator => write!(f, "operator"),
            SyntaxShape::RowCondition => write!(f, "condition"),
            SyntaxShape::MathExpression => write!(f, "variable"),
            SyntaxShape::Variable => write!(f, "var"),
            SyntaxShape::VarWithOptType => write!(f, "vardecl"),
            SyntaxShape::Signature => write!(f, "signature"),
            SyntaxShape::Expression => write!(f, "expression"),
            SyntaxShape::Boolean => write!(f, "bool"),
            SyntaxShape::Custom(x, _) => write!(f, "custom<{}>", x),
        }
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
    }
}
