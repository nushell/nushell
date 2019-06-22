crate mod baseline_parse;
crate mod baseline_parse_tokens;
crate mod binary;
crate mod named;
crate mod path;

use crate::parser::{Span, Spanned, Unit};
use derive_new::new;
use getset::Getters;

crate use baseline_parse::baseline_parse_single_token;
crate use baseline_parse_tokens::{baseline_parse_next_expr, ExpressionKindHint};
crate use binary::Binary;
crate use named::NamedArguments;
crate use path::Path;

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
    Block(Box<Expression>),
    Path(Box<Path>),

    #[allow(unused)]
    Boolean(bool),
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

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Literal {
    Integer(i64),
    Size(i64, Unit),
    String(Span),
    Bare,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Variable {
    It(Span),
    Other(Span),
}
