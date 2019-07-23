crate mod baseline_parse;
crate mod baseline_parse_tokens;
crate mod binary;
crate mod named;
crate mod path;

use crate::evaluate::Scope;
use crate::parser::{registry, Span, Spanned, Unit};
use crate::prelude::*;
use derive_new::new;
use getset::Getters;
use serde_derive::{Deserialize, Serialize};
use std::fmt;

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

#[derive(Debug, Clone, Eq, PartialEq, Getters, Serialize, Deserialize, new)]
pub struct Call {
    #[get = "crate"]
    head: Box<Expression>,
    #[get = "crate"]
    positional: Option<Vec<Expression>>,
    #[get = "crate"]
    named: Option<NamedArguments>,
}

impl Call {
    pub fn evaluate(
        &self,
        registry: &registry::CommandRegistry,
        scope: &Scope,
        source: &Text,
    ) -> Result<registry::EvaluatedArgs, ShellError> {
        registry::evaluate_args(self, registry, scope, source)
    }
}

impl ToDebug for Call {
    fn fmt_debug(&self, f: &mut fmt::Formatter, source: &str) -> fmt::Result {
        write!(f, "({}", self.head.debug(source))?;

        if let Some(positional) = &self.positional {
            write!(f, " ")?;
            write!(
                f,
                "{}",
                &itertools::join(positional.iter().map(|p| p.debug(source)), " ")
            )?;
        }

        if let Some(named) = &self.named {
            write!(f, "{}", named.debug(source))?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum RawExpression {
    Literal(Literal),
    Synthetic(Synthetic),
    Variable(Variable),
    Binary(Box<Binary>),
    Block(Vec<Expression>),
    Path(Box<Path>),

    #[allow(unused)]
    Boolean(bool),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum Synthetic {
    String(String),
}

impl Synthetic {
    pub fn type_name(&self) -> &'static str {
        match self {
            Synthetic::String(_) => "string",
        }
    }
}

impl RawExpression {
    pub fn type_name(&self) -> &'static str {
        match self {
            RawExpression::Literal(literal) => literal.type_name(),
            RawExpression::Synthetic(synthetic) => synthetic.type_name(),
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
    crate fn int(i: impl Into<i64>, span: impl Into<Span>) -> Expression {
        Spanned::from_item(RawExpression::Literal(Literal::Integer(i.into())), span)
    }

    crate fn size(i: impl Into<i64>, unit: impl Into<Unit>, span: impl Into<Span>) -> Expression {
        Spanned::from_item(
            RawExpression::Literal(Literal::Size(i.into(), unit.into())),
            span,
        )
    }

    crate fn synthetic_string(s: impl Into<String>) -> Expression {
        RawExpression::Synthetic(Synthetic::String(s.into())).spanned_unknown()
    }

    crate fn string(inner: impl Into<Span>, outer: impl Into<Span>) -> Expression {
        Spanned::from_item(
            RawExpression::Literal(Literal::String(inner.into())),
            outer.into(),
        )
    }

    crate fn bare(span: impl Into<Span>) -> Expression {
        Spanned::from_item(RawExpression::Literal(Literal::Bare), span.into())
    }

    crate fn variable(inner: impl Into<Span>, outer: impl Into<Span>) -> Expression {
        Spanned::from_item(
            RawExpression::Variable(Variable::Other(inner.into())),
            outer.into(),
        )
    }

    crate fn it_variable(inner: impl Into<Span>, outer: impl Into<Span>) -> Expression {
        Spanned::from_item(
            RawExpression::Variable(Variable::It(inner.into())),
            outer.into(),
        )
    }
}

impl ToDebug for Expression {
    fn fmt_debug(&self, f: &mut fmt::Formatter, source: &str) -> fmt::Result {
        match self.item() {
            RawExpression::Literal(l) => write!(f, "{}", l.spanned(self.span()).debug(source)),
            RawExpression::Synthetic(Synthetic::String(s)) => write!(f, "{:?}", s),
            RawExpression::Variable(Variable::It(_)) => write!(f, "$it"),
            RawExpression::Variable(Variable::Other(s)) => write!(f, "${}", s.slice(source)),
            RawExpression::Binary(b) => write!(f, "{}", b.debug(source)),
            RawExpression::Block(exprs) => {
                write!(f, "{{ ")?;

                for expr in exprs {
                    write!(f, "{} ", expr.debug(source))?;
                }

                write!(f, "}}")
            }
            RawExpression::Path(p) => write!(f, "{}", p.debug(source)),
            RawExpression::Boolean(true) => write!(f, "$yes"),
            RawExpression::Boolean(false) => write!(f, "$no"),
        }
    }
}

impl From<Spanned<Path>> for Expression {
    fn from(path: Spanned<Path>) -> Expression {
        path.map(|p| RawExpression::Path(Box::new(p)))
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum Literal {
    Integer(i64),
    Size(i64, Unit),
    String(Span),
    Bare,
}

impl ToDebug for Spanned<&Literal> {
    fn fmt_debug(&self, f: &mut fmt::Formatter, source: &str) -> fmt::Result {
        match self.item() {
            Literal::Integer(int) => write!(f, "{}", *int),
            Literal::Size(int, unit) => write!(f, "{}{:?}", *int, unit),
            Literal::String(span) => write!(f, "{}", span.slice(source)),
            Literal::Bare => write!(f, "{}", self.span().slice(source)),
        }
    }
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

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum Variable {
    It(Span),
    Other(Span),
}
