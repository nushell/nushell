pub(crate) mod baseline_parse;
pub(crate) mod binary;
pub(crate) mod expand_external_tokens;
pub(crate) mod external_command;
pub(crate) mod named;
pub(crate) mod path;
pub(crate) mod range;
pub(crate) mod signature;
pub mod syntax_shape;
pub(crate) mod tokens_iterator;

use crate::hir::syntax_shape::Member;
use crate::parse::operator::CompareOperator;
use crate::parse::parser::Number;
use crate::parse::unit::Unit;
use derive_new::new;
use getset::Getters;
use nu_protocol::{PathMember, ShellTypeName};
use nu_source::{
    b, DebugDocBuilder, HasSpan, PrettyDebug, PrettyDebugWithSource, Span, Spanned, SpannedItem,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::parse::tokens::RawNumber;

pub(crate) use self::binary::Binary;
pub(crate) use self::path::Path;
pub(crate) use self::range::Range;
pub(crate) use self::syntax_shape::ExpandContext;
pub(crate) use self::tokens_iterator::TokensIterator;

pub use self::external_command::ExternalCommand;
pub use self::named::{NamedArguments, NamedValue};

#[derive(Debug, Clone)]
pub struct Signature {
    unspanned: nu_protocol::Signature,
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

#[derive(Debug, Clone, Eq, PartialEq, Getters, Serialize, Deserialize, new)]
pub struct Call {
    #[get = "pub(crate)"]
    pub head: Box<Expression>,
    #[get = "pub(crate)"]
    pub positional: Option<Vec<Expression>>,
    #[get = "pub(crate)"]
    pub named: Option<NamedArguments>,
    pub span: Span,
}

impl PrettyDebugWithSource for Call {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        b::delimit(
            "(",
            self.head.pretty_debug(source)
                + b::preceded_option(
                    Some(b::space()),
                    self.positional.as_ref().map(|pos| {
                        b::intersperse(pos.iter().map(|expr| expr.pretty_debug(source)), b::space())
                    }),
                )
                + b::preceded_option(
                    Some(b::space()),
                    self.named.as_ref().map(|named| named.pretty_debug(source)),
                ),
            ")",
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum RawExpression {
    Literal(Literal),
    ExternalWord,
    Synthetic(Synthetic),
    Variable(Variable),
    Binary(Box<Binary>),
    Range(Box<Range>),
    Block(Vec<Expression>),
    List(Vec<Expression>),
    Path(Box<Path>),

    FilePath(PathBuf),
    ExternalCommand(ExternalCommand),
    Command(Span),

    Boolean(bool),
}

impl ShellTypeName for RawExpression {
    fn type_name(&self) -> &'static str {
        match self {
            RawExpression::Literal(literal) => literal.type_name(),
            RawExpression::Synthetic(synthetic) => synthetic.type_name(),
            RawExpression::Command(..) => "command",
            RawExpression::ExternalWord => "external word",
            RawExpression::FilePath(..) => "file path",
            RawExpression::Variable(..) => "variable",
            RawExpression::List(..) => "list",
            RawExpression::Binary(..) => "binary",
            RawExpression::Range(..) => "range",
            RawExpression::Block(..) => "block",
            RawExpression::Path(..) => "variable path",
            RawExpression::Boolean(..) => "boolean",
            RawExpression::ExternalCommand(..) => "external",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum Synthetic {
    String(String),
}

impl ShellTypeName for Synthetic {
    fn type_name(&self) -> &'static str {
        match self {
            Synthetic::String(_) => "string",
        }
    }
}

impl RawExpression {
    pub fn into_expr(self, span: impl Into<Span>) -> Expression {
        Expression {
            expr: self,
            span: span.into(),
        }
    }

    pub fn into_unspanned_expr(self) -> Expression {
        Expression {
            expr: self,
            span: Span::unknown(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct Expression {
    pub expr: RawExpression,
    pub span: Span,
}

impl std::ops::Deref for Expression {
    type Target = RawExpression;

    fn deref(&self) -> &RawExpression {
        &self.expr
    }
}

impl HasSpan for Expression {
    fn span(&self) -> Span {
        self.span
    }
}

impl PrettyDebugWithSource for Expression {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        match &self.expr {
            RawExpression::Literal(literal) => literal.spanned(self.span).pretty_debug(source),
            RawExpression::ExternalWord => {
                b::typed("external word", b::primitive(self.span.slice(source)))
            }
            RawExpression::Synthetic(s) => match s {
                Synthetic::String(s) => b::typed("synthetic", b::primitive(format!("{:?}", s))),
            },
            RawExpression::Variable(_) => b::keyword(self.span.slice(source)),
            RawExpression::Binary(binary) => binary.pretty_debug(source),
            RawExpression::Range(range) => range.pretty_debug(source),
            RawExpression::Block(_) => b::opaque("block"),
            RawExpression::List(list) => b::delimit(
                "[",
                b::intersperse(
                    list.iter().map(|item| item.pretty_debug(source)),
                    b::space(),
                ),
                "]",
            ),
            RawExpression::Path(path) => path.pretty_debug(source),
            RawExpression::FilePath(path) => b::typed("path", b::primitive(path.display())),
            RawExpression::ExternalCommand(external) => b::typed(
                "external command",
                b::primitive(external.name.slice(source)),
            ),
            RawExpression::Command(command) => {
                b::typed("command", b::primitive(command.slice(source)))
            }
            RawExpression::Boolean(boolean) => match boolean {
                true => b::primitive("$yes"),
                false => b::primitive("$no"),
            },
        }
    }
}

impl Expression {
    pub fn number(i: impl Into<Number>, span: impl Into<Span>) -> Expression {
        let span = span.into();

        RawExpression::Literal(RawLiteral::Number(i.into()).into_literal(span)).into_expr(span)
    }

    pub fn size(i: impl Into<Number>, unit: impl Into<Unit>, span: impl Into<Span>) -> Expression {
        let span = span.into();

        RawExpression::Literal(RawLiteral::Size(i.into(), unit.into()).into_literal(span))
            .into_expr(span)
    }

    pub fn synthetic_string(s: impl Into<String>) -> Expression {
        RawExpression::Synthetic(Synthetic::String(s.into())).into_unspanned_expr()
    }

    pub fn string(inner: impl Into<Span>, outer: impl Into<Span>) -> Expression {
        let outer = outer.into();

        RawExpression::Literal(RawLiteral::String(inner.into()).into_literal(outer))
            .into_expr(outer)
    }

    pub fn column_path(members: Vec<Member>, span: impl Into<Span>) -> Expression {
        let span = span.into();

        RawExpression::Literal(RawLiteral::ColumnPath(members).into_literal(span)).into_expr(span)
    }

    pub fn path(
        head: Expression,
        tail: Vec<impl Into<PathMember>>,
        span: impl Into<Span>,
    ) -> Expression {
        let tail = tail.into_iter().map(|t| t.into()).collect();
        RawExpression::Path(Box::new(Path::new(head, tail))).into_expr(span.into())
    }

    pub fn dot_member(head: Expression, next: impl Into<PathMember>) -> Expression {
        let Expression { expr: item, span } = head;
        let next = next.into();

        let new_span = head.span.until(next.span);

        match item {
            RawExpression::Path(path) => {
                let (head, mut tail) = path.parts();

                tail.push(next);
                Expression::path(head, tail, new_span)
            }

            other => Expression::path(other.into_expr(span), vec![next], new_span),
        }
    }

    pub fn infix(
        left: Expression,
        op: Spanned<impl Into<CompareOperator>>,
        right: Expression,
    ) -> Expression {
        let new_span = left.span.until(right.span);

        RawExpression::Binary(Box::new(Binary::new(left, op.map(|o| o.into()), right)))
            .into_expr(new_span)
    }

    pub fn range(left: Expression, op: Span, right: Expression) -> Expression {
        let new_span = left.span.until(right.span);

        RawExpression::Range(Box::new(Range::new(left, op, right))).into_expr(new_span)
    }

    pub fn file_path(path: impl Into<PathBuf>, outer: impl Into<Span>) -> Expression {
        RawExpression::FilePath(path.into()).into_expr(outer)
    }

    pub fn list(list: Vec<Expression>, span: impl Into<Span>) -> Expression {
        RawExpression::List(list).into_expr(span)
    }

    pub fn bare(span: impl Into<Span>) -> Expression {
        let span = span.into();

        RawExpression::Literal(RawLiteral::Bare.into_literal(span)).into_expr(span)
    }

    pub fn pattern(inner: impl Into<String>, outer: impl Into<Span>) -> Expression {
        let outer = outer.into();

        RawExpression::Literal(RawLiteral::GlobPattern(inner.into()).into_literal(outer))
            .into_expr(outer)
    }

    pub fn variable(inner: impl Into<Span>, outer: impl Into<Span>) -> Expression {
        RawExpression::Variable(Variable::Other(inner.into())).into_expr(outer)
    }

    pub fn external_command(inner: impl Into<Span>, outer: impl Into<Span>) -> Expression {
        RawExpression::ExternalCommand(ExternalCommand::new(inner.into())).into_expr(outer)
    }

    pub fn it_variable(inner: impl Into<Span>, outer: impl Into<Span>) -> Expression {
        RawExpression::Variable(Variable::It(inner.into())).into_expr(outer)
    }
}

impl From<Spanned<Path>> for Expression {
    fn from(path: Spanned<Path>) -> Expression {
        RawExpression::Path(Box::new(path.item)).into_expr(path.span)
    }
}

/// Literals are expressions that are:
///
/// 1. Copy
/// 2. Can be evaluated without additional context
/// 3. Evaluation cannot produce an error
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum RawLiteral {
    Number(Number),
    Size(Number, Unit),
    String(Span),
    GlobPattern(String),
    ColumnPath(Vec<Member>),
    Bare,
}

impl RawLiteral {
    pub fn into_literal(self, span: impl Into<Span>) -> Literal {
        Literal {
            literal: self,
            span: span.into(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct Literal {
    pub literal: RawLiteral,
    pub span: Span,
}

impl ShellTypeName for Literal {
    fn type_name(&self) -> &'static str {
        match &self.literal {
            RawLiteral::Number(..) => "number",
            RawLiteral::Size(..) => "size",
            RawLiteral::String(..) => "string",
            RawLiteral::ColumnPath(..) => "column path",
            RawLiteral::Bare => "string",
            RawLiteral::GlobPattern(_) => "pattern",
        }
    }
}

impl PrettyDebugWithSource for Literal {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        match &self.literal {
            RawLiteral::Number(number) => number.pretty(),
            RawLiteral::Size(number, unit) => (number.pretty() + unit.pretty()).group(),
            RawLiteral::String(string) => b::primitive(format!("{:?}", string.slice(source))),
            RawLiteral::GlobPattern(pattern) => b::typed("pattern", b::primitive(pattern)),
            RawLiteral::ColumnPath(path) => b::typed(
                "column path",
                b::intersperse_with_source(path.iter(), b::space(), source),
            ),
            RawLiteral::Bare => b::primitive(self.span.slice(source)),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum Variable {
    It(Span),
    Other(Span),
}
