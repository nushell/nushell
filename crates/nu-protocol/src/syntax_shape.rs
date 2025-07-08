use crate::{DeclId, Type};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// The syntactic shapes that describe how a sequence should be parsed.
///
/// This extends beyond [`Type`] which describes how [`Value`](crate::Value)s are represented.
/// `SyntaxShape`s can describe the parsing rules for arguments to a command.
/// e.g. [`SyntaxShape::GlobPattern`]/[`SyntaxShape::Filepath`] serve the completer,
/// but don't have an associated [`Value`](crate::Value)
/// There are additional `SyntaxShape`s that only make sense in particular expressions or keywords
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

    /// A [`SyntaxShape`] with custom completion logic
    CompleterWrapper(Box<SyntaxShape>, DeclId),

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

    /// A (typically) string argument that follows external command argument parsing rules.
    ///
    /// Filepaths are expanded if unquoted, globs are allowed, and quotes embedded within unknown
    /// args are unquoted.
    ExternalArgument,

    /// A filepath is allowed
    Filepath,

    /// A filesize value is allowed, eg `10kb`
    Filesize,

    /// A floating point value, eg `1.0`
    Float,

    /// A dotted path including the variable to access items
    ///
    /// Fully qualified
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

    /// A block of matches, used by `match`
    MatchBlock,

    /// Nothing
    Nothing,

    /// Only a numeric (integer or float) value is allowed
    Number,

    /// One of a list of possible items, checked in order
    OneOf(Vec<SyntaxShape>),

    /// An operator, eg `+`
    Operator,

    /// A range is allowed (eg, `1..3`)
    Range,

    /// A record value, eg `{x: 1, y: 2}`
    Record(Vec<(String, SyntaxShape)>),

    /// A math expression which expands shorthand forms on the lefthand side, eg `foo > 1`
    /// The shorthand allows us to more easily reach columns inside of the row being passed in
    RowCondition,

    /// A signature for a definition, `[x:int, --foo]`
    Signature,

    /// Strings and string-like bare words are allowed
    String,

    /// A table is allowed, eg `[[first, second]; [1, 2]]`
    Table(Vec<(String, SyntaxShape)>),

    /// A variable with optional type, `x` or `x: int`
    VarWithOptType,
}

impl SyntaxShape {
    /// If possible provide the associated concrete [`Type`]
    ///
    /// Note: Some [`SyntaxShape`]s don't have a corresponding [`Value`](crate::Value)
    /// Here we currently return [`Type::Any`]
    ///
    /// ```rust
    /// use nu_protocol::{SyntaxShape, Type};
    /// let non_value = SyntaxShape::ImportPattern;
    /// assert_eq!(non_value.to_type(), Type::Any);
    /// ```
    pub fn to_type(&self) -> Type {
        let mk_ty = |tys: &[(String, SyntaxShape)]| {
            tys.iter()
                .map(|(key, val)| (key.clone(), val.to_type()))
                .collect()
        };

        match self {
            SyntaxShape::Any => Type::Any,
            SyntaxShape::Block => Type::Block,
            SyntaxShape::Closure(_) => Type::Closure,
            SyntaxShape::Binary => Type::Binary,
            SyntaxShape::CellPath => Type::Any,
            SyntaxShape::CompleterWrapper(inner, _) => inner.to_type(),
            SyntaxShape::DateTime => Type::Date,
            SyntaxShape::Duration => Type::Duration,
            SyntaxShape::Expression => Type::Any,
            SyntaxShape::ExternalArgument => Type::Any,
            SyntaxShape::Filepath => Type::String,
            SyntaxShape::Directory => Type::String,
            SyntaxShape::Float => Type::Float,
            SyntaxShape::Filesize => Type::Filesize,
            SyntaxShape::FullCellPath => Type::Any,
            SyntaxShape::GlobPattern => Type::Glob,
            SyntaxShape::Error => Type::Error,
            SyntaxShape::ImportPattern => Type::Any,
            SyntaxShape::Int => Type::Int,
            SyntaxShape::List(x) => {
                let contents = x.to_type();
                Type::List(Box::new(contents))
            }
            SyntaxShape::Keyword(_, expr) => expr.to_type(),
            SyntaxShape::MatchBlock => Type::Any,
            SyntaxShape::MathExpression => Type::Any,
            SyntaxShape::Nothing => Type::Nothing,
            SyntaxShape::Number => Type::Number,
            SyntaxShape::OneOf(_) => Type::Any,
            SyntaxShape::Operator => Type::Any,
            SyntaxShape::Range => Type::Range,
            SyntaxShape::Record(entries) => Type::Record(mk_ty(entries)),
            SyntaxShape::RowCondition => Type::Bool,
            SyntaxShape::Boolean => Type::Bool,
            SyntaxShape::Signature => Type::Any,
            SyntaxShape::String => Type::String,
            SyntaxShape::Table(columns) => Type::Table(mk_ty(columns)),
            SyntaxShape::VarWithOptType => Type::Any,
        }
    }
}

impl Display for SyntaxShape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mk_fmt = |tys: &[(String, SyntaxShape)]| -> String {
            tys.iter()
                .map(|(x, y)| format!("{x}: {y}"))
                .collect::<Vec<String>>()
                .join(", ")
        };

        match self {
            SyntaxShape::Keyword(kw, shape) => {
                write!(f, "\"{}\" {}", String::from_utf8_lossy(kw), shape)
            }
            SyntaxShape::Any => write!(f, "any"),
            SyntaxShape::String => write!(f, "string"),
            SyntaxShape::CellPath => write!(f, "cell-path"),
            SyntaxShape::FullCellPath => write!(f, "cell-path"),
            SyntaxShape::Number => write!(f, "number"),
            SyntaxShape::Range => write!(f, "range"),
            SyntaxShape::Int => write!(f, "int"),
            SyntaxShape::Float => write!(f, "float"),
            SyntaxShape::Filepath => write!(f, "path"),
            SyntaxShape::Directory => write!(f, "directory"),
            SyntaxShape::GlobPattern => write!(f, "glob"),
            SyntaxShape::ImportPattern => write!(f, "import"),
            SyntaxShape::Block => write!(f, "block"),
            SyntaxShape::Closure(args) => {
                if let Some(args) = args {
                    let arg_vec: Vec<_> = args.iter().map(|x| x.to_string()).collect();
                    let arg_string = arg_vec.join(", ");
                    write!(f, "closure({arg_string})")
                } else {
                    write!(f, "closure()")
                }
            }
            SyntaxShape::Binary => write!(f, "binary"),
            SyntaxShape::List(x) => write!(f, "list<{x}>"),
            SyntaxShape::Table(columns) => {
                if columns.is_empty() {
                    write!(f, "table")
                } else {
                    write!(f, "table<{}>", mk_fmt(columns))
                }
            }
            SyntaxShape::Record(entries) => {
                if entries.is_empty() {
                    write!(f, "record")
                } else {
                    write!(f, "record<{}>", mk_fmt(entries))
                }
            }
            SyntaxShape::Filesize => write!(f, "filesize"),
            SyntaxShape::Duration => write!(f, "duration"),
            SyntaxShape::DateTime => write!(f, "datetime"),
            SyntaxShape::Operator => write!(f, "operator"),
            SyntaxShape::RowCondition => write!(f, "condition"),
            SyntaxShape::MathExpression => write!(f, "variable"),
            SyntaxShape::VarWithOptType => write!(f, "vardecl"),
            SyntaxShape::Signature => write!(f, "signature"),
            SyntaxShape::MatchBlock => write!(f, "match-block"),
            SyntaxShape::Expression => write!(f, "expression"),
            SyntaxShape::ExternalArgument => write!(f, "external-argument"),
            SyntaxShape::Boolean => write!(f, "bool"),
            SyntaxShape::Error => write!(f, "error"),
            SyntaxShape::CompleterWrapper(x, _) => write!(f, "completable<{x}>"),
            SyntaxShape::OneOf(list) => {
                write!(f, "oneof<")?;
                if let Some((last, rest)) = list.split_last() {
                    for ty in rest {
                        write!(f, "{ty}, ")?;
                    }
                    write!(f, "{last}")?;
                }
                write!(f, ">")
            }
            SyntaxShape::Nothing => write!(f, "nothing"),
        }
    }
}
