use std::path::PathBuf;

use nu_source::{DebugDocBuilder, HasSpan, PrettyDebugWithSource};
use nu_source::{Span, Spanned};

use bigdecimal::BigDecimal;
use indexmap::IndexMap;
use num_bigint::BigInt;

#[derive(Debug, Clone)]
pub struct ExternalCommand {
    pub name: Spanned<String>,
    pub args: Vec<Spanned<String>>,
}

#[derive(Debug, Clone, Copy)]
pub enum Unit {
    // Filesize units
    Byte,
    Kilobyte,
    Megabyte,
    Gigabyte,
    Terabyte,
    Petabyte,

    // Duration units
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Year,
}

#[derive(Debug, Clone)]
pub enum Member {
    String(/* outer */ Span, /* inner */ Span),
    Int(BigInt, Span),
    Bare(Spanned<String>),
}

#[derive(Debug, Clone)]
pub enum Number {
    Int(BigInt),
    Decimal(BigDecimal),
}
#[derive(Debug, Clone)]
pub struct SpannedExpression {
    pub expr: Expression,
    pub span: Span,
}

impl SpannedExpression {
    pub fn new(expr: Expression, span: Span) -> SpannedExpression {
        SpannedExpression { expr, span }
    }
}

#[derive(Debug, Clone)]
pub enum Variable {
    It(Span),
    Other(String, Span),
}

#[derive(Debug, Clone)]
pub enum CompareOperator {
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessThanOrEqual,
    GreaterThanOrEqual,
    Contains,
    NotContains,
}

#[derive(Debug, Clone)]
pub struct Binary {
    pub left: SpannedExpression,
    pub op: Spanned<CompareOperator>,
    pub right: SpannedExpression,
}

#[derive(Debug, Clone)]
pub enum Synthetic {
    String(String),
}

#[derive(Debug, Clone)]
pub struct Range {
    pub left: SpannedExpression,
    pub dotdot: Span,
    pub right: SpannedExpression,
}

#[derive(Debug, Clone)]
pub enum Literal {
    Number(Number),
    Size(Number, Unit),
    Operator(CompareOperator),
    String(String),
    GlobPattern(String),
    ColumnPath(Vec<Member>),
    Bare,
}

#[derive(Debug, Clone)]
pub struct Path {
    pub head: SpannedExpression,
    pub tail: Vec<Member>,
}

#[derive(Debug, Clone)]
pub enum Expression {
    Literal(Literal),
    ExternalWord,
    Synthetic(Synthetic),
    Variable(Variable),
    Binary(Box<Binary>),
    Range(Box<Range>),
    Block(Vec<SpannedExpression>),
    List(Vec<SpannedExpression>),
    Path(Box<Path>),

    FilePath(PathBuf),
    ExternalCommand(ExternalCommand),
    Command(Span),

    Boolean(bool),

    // Trying this approach out: if we let parsing always be infallible
    // we can use the same parse and just place bad token markers in the output
    // We can later throw an error if we try to process them further.
    Garbage,
}

impl Expression {
    pub fn integer(i: i64) -> Expression {
        Expression::Literal(Literal::Number(Number::Int(BigInt::from(i))))
    }

    pub fn decimal(f: f64) -> Expression {
        Expression::Literal(Literal::Number(Number::Decimal(BigDecimal::from(f))))
    }

    pub fn string(s: String) -> Expression {
        Expression::Literal(Literal::String(s))
    }

    pub fn operator(operator: CompareOperator) -> Expression {
        Expression::Literal(Literal::Operator(operator))
    }

    pub fn range(left: SpannedExpression, dotdot: Span, right: SpannedExpression) -> Expression {
        Expression::Range(Box::new(Range {
            left,
            dotdot,
            right,
        }))
    }

    pub fn pattern(p: String) -> Expression {
        Expression::Literal(Literal::GlobPattern(p))
    }

    pub fn file_path(file_path: PathBuf) -> Expression {
        Expression::FilePath(file_path)
    }

    pub fn column_path(head: SpannedExpression, tail: Vec<Member>) -> Expression {
        Expression::Path(Box::new(Path { head, tail }))
    }

    pub fn unit(i: i64, unit: Unit) -> Expression {
        Expression::Literal(Literal::Size(Number::Int(BigInt::from(i)), unit))
    }

    pub fn variable(v: String, span: Span) -> Expression {
        if v == "$it" {
            Expression::Variable(Variable::It(span))
        } else {
            Expression::Variable(Variable::Other(v, span))
        }
    }
}

#[derive(Debug, Clone)]
pub enum NamedValue {
    AbsentSwitch,
    PresentSwitch(Span),
    AbsentValue,
    Value(Span, SpannedExpression),
}

#[derive(Debug, Clone)]
pub struct Call {
    pub head: SpannedExpression,
    pub positional: Vec<SpannedExpression>,
    pub named: IndexMap<String, NamedValue>,
    pub span: Span,
}

impl Call {
    pub fn new(head: SpannedExpression, span: Span) -> Call {
        Call {
            head,
            positional: vec![],
            named: IndexMap::new(),
            span,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Delimiter {
    Paren,
    Brace,
    Square,
}

#[derive(Debug, Copy, Clone)]
pub enum FlatShape {
    OpenDelimiter(Delimiter),
    CloseDelimiter(Delimiter),
    Type,
    Identifier,
    ItVariable,
    Variable,
    CompareOperator,
    Dot,
    DotDot,
    InternalCommand,
    ExternalCommand,
    ExternalWord,
    BareMember,
    StringMember,
    String,
    Path,
    Word,
    Keyword,
    Pipe,
    GlobPattern,
    Flag,
    ShorthandFlag,
    Int,
    Decimal,
    Garbage,
    Whitespace,
    Separator,
    Comment,
    Size { number: Span, unit: Span },
}

#[derive(Debug, Clone)]
pub struct Signature {
    pub(crate) unspanned: nu_protocol::Signature,
    span: Span,
}

impl Signature {
    pub fn new(unspanned: nu_protocol::Signature, span: impl Into<Span>) -> Signature {
        Signature {
            unspanned,
            span: span.into(),
        }
    }
}

impl HasSpan for Signature {
    fn span(&self) -> Span {
        self.span
    }
}

impl PrettyDebugWithSource for Signature {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        self.unspanned.pretty_debug(source)
    }
}
