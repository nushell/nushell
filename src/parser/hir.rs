crate mod baseline_parse;
crate mod baseline_parse_tokens;
crate mod binary;
crate mod named;
crate mod path;

use crate::parser::{Span, Spanned, Unit};
use derive_new::new;
use getset::Getters;

crate use baseline_parse::{baseline_parse_single_token, baseline_parse_token_as_string};
crate use baseline_parse_tokens::{baseline_parse_next_expr, SyntaxType, TokensIterator};
crate use binary::Binary;
crate use named::NamedArguments;
crate use path::Path;

pub fn path(head: impl Into<Expression>, tail: Vec<Spanned<impl Into<String>>>) -> Path {
    Path::new(
        head.into(),
        tail.into_iter()
            .map(|item| item.map(|string| string.into()))
            .collect(),
    )
}

#[derive(Debug, Clone, Eq, PartialEq, Getters, new)]
pub struct Call {
    #[get = "crate"]
    head: Box<Expression>,
    #[get = "crate"]
    positional: Option<Vec<Expression>>,
    #[get = "crate"]
    named: Option<NamedArguments>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum RawExpression {
    Literal(Literal),
    Variable(Variable),
    Binary(Box<Binary>),
    Block(Vec<Expression>),
    Path(Box<Path>),

    #[allow(unused)]
    Boolean(bool),
}

impl RawExpression {
    pub fn type_name(&self) -> &'static str {
        match self {
            RawExpression::Literal(literal) => literal.type_name(),
            RawExpression::Variable(..) => "variable",
            RawExpression::Binary(..) => "binary",
            RawExpression::Block(..) => "block",
            RawExpression::Path(..) => "path",
            RawExpression::Boolean(..) => "boolean",
        }
    }
}

pub type Expression = Spanned<RawExpression>;

impl Expression {
    fn int(i: impl Into<i64>, span: impl Into<Span>) -> Expression {
        Spanned::from_item(RawExpression::Literal(Literal::Integer(i.into())), span)
    }

    fn size(i: impl Into<i64>, unit: impl Into<Unit>, span: impl Into<Span>) -> Expression {
        Spanned::from_item(
            RawExpression::Literal(Literal::Size(i.into(), unit.into())),
            span,
        )
    }

    fn string(inner: impl Into<Span>, outer: impl Into<Span>) -> Expression {
        Spanned::from_item(
            RawExpression::Literal(Literal::String(inner.into())),
            outer.into(),
        )
    }

    fn bare(span: impl Into<Span>) -> Expression {
        Spanned::from_item(RawExpression::Literal(Literal::Bare), span.into())
    }

    fn variable(inner: impl Into<Span>, outer: impl Into<Span>) -> Expression {
        Spanned::from_item(
            RawExpression::Variable(Variable::Other(inner.into())),
            outer.into(),
        )
    }

    fn it_variable(inner: impl Into<Span>, outer: impl Into<Span>) -> Expression {
        Spanned::from_item(
            RawExpression::Variable(Variable::It(inner.into())),
            outer.into(),
        )
    }
}

impl From<Spanned<Path>> for Expression {
    fn from(path: Spanned<Path>) -> Expression {
        path.map(|p| RawExpression::Path(Box::new(p)))
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Literal {
    Integer(i64),
    Size(i64, Unit),
    String(Span),
    Bare,
}

impl Literal {
    fn type_name(&self) -> &'static str {
        match self {
            Literal::Integer(_) => "integer",
            Literal::Size(..) => "size",
            Literal::String(..) => "string",
            Literal::Bare => "string",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Variable {
    It(Span),
    Other(Span),
}
