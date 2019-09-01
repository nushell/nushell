pub(crate) mod baseline_parse;
pub(crate) mod baseline_parse_tokens;
pub(crate) mod binary;
pub(crate) mod external_command;
pub(crate) mod named;
pub(crate) mod path;

use crate::parser::{registry, Unit};
use crate::prelude::*;
use derive_new::new;
use getset::Getters;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

use crate::evaluate::Scope;

pub(crate) use self::baseline_parse::{
    baseline_parse_single_token, baseline_parse_token_as_number, baseline_parse_token_as_path,
    baseline_parse_token_as_string,
};
pub(crate) use self::baseline_parse_tokens::{baseline_parse_next_expr, TokensIterator};
pub(crate) use self::binary::Binary;
pub(crate) use self::external_command::ExternalCommand;
pub(crate) use self::named::NamedArguments;
pub(crate) use self::path::Path;

pub use self::baseline_parse_tokens::SyntaxType;

pub fn path(head: impl Into<Expression>, tail: Vec<Tagged<impl Into<String>>>) -> Path {
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
    pub head: Box<Expression>,
    #[get = "crate"]
    pub positional: Option<Vec<Expression>>,
    #[get = "crate"]
    pub named: Option<NamedArguments>,
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

        write!(f, ")")?;

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
    List(Vec<Expression>),
    Path(Box<Path>),
    FilePath(PathBuf),
    ExternalCommand(ExternalCommand),

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
            RawExpression::FilePath(..) => "filepath",
            RawExpression::Variable(..) => "variable",
            RawExpression::List(..) => "list",
            RawExpression::Binary(..) => "binary",
            RawExpression::Block(..) => "block",
            RawExpression::Path(..) => "path",
            RawExpression::Boolean(..) => "boolean",
            RawExpression::ExternalCommand(..) => "external",
        }
    }
}

pub type Expression = Tagged<RawExpression>;

impl Expression {
    pub(crate) fn number(i: impl Into<Number>, span: impl Into<Span>) -> Expression {
        Tagged::from_simple_spanned_item(RawExpression::Literal(Literal::Number(i.into())), span)
    }

    pub(crate) fn size(
        i: impl Into<Number>,
        unit: impl Into<Unit>,
        span: impl Into<Span>,
    ) -> Expression {
        Tagged::from_simple_spanned_item(
            RawExpression::Literal(Literal::Size(i.into(), unit.into())),
            span,
        )
    }

    pub(crate) fn synthetic_string(s: impl Into<String>) -> Expression {
        RawExpression::Synthetic(Synthetic::String(s.into())).tagged_unknown()
    }

    pub(crate) fn string(inner: impl Into<Span>, outer: impl Into<Span>) -> Expression {
        Tagged::from_simple_spanned_item(
            RawExpression::Literal(Literal::String(inner.into())),
            outer.into(),
        )
    }

    pub(crate) fn file_path(path: impl Into<PathBuf>, outer: impl Into<Span>) -> Expression {
        Tagged::from_simple_spanned_item(RawExpression::FilePath(path.into()), outer.into())
    }

    pub(crate) fn bare(span: impl Into<Span>) -> Expression {
        Tagged::from_simple_spanned_item(RawExpression::Literal(Literal::Bare), span.into())
    }

    pub(crate) fn variable(inner: impl Into<Span>, outer: impl Into<Span>) -> Expression {
        Tagged::from_simple_spanned_item(
            RawExpression::Variable(Variable::Other(inner.into())),
            outer.into(),
        )
    }

    pub(crate) fn external_command(inner: impl Into<Span>, outer: impl Into<Span>) -> Expression {
        Tagged::from_simple_spanned_item(
            RawExpression::ExternalCommand(ExternalCommand::new(inner.into())),
            outer.into(),
        )
    }

    pub(crate) fn it_variable(inner: impl Into<Span>, outer: impl Into<Span>) -> Expression {
        Tagged::from_simple_spanned_item(
            RawExpression::Variable(Variable::It(inner.into())),
            outer.into(),
        )
    }
}

impl ToDebug for Expression {
    fn fmt_debug(&self, f: &mut fmt::Formatter, source: &str) -> fmt::Result {
        match self.item() {
            RawExpression::Literal(l) => l.tagged(self.span()).fmt_debug(f, source),
            RawExpression::FilePath(p) => write!(f, "{}", p.display()),
            RawExpression::Synthetic(Synthetic::String(s)) => write!(f, "{:?}", s),
            RawExpression::Variable(Variable::It(_)) => write!(f, "$it"),
            RawExpression::Variable(Variable::Other(s)) => write!(f, "${}", s.slice(source)),
            RawExpression::Binary(b) => write!(f, "{}", b.debug(source)),
            RawExpression::ExternalCommand(c) => write!(f, "^{}", c.name().slice(source)),
            RawExpression::Block(exprs) => {
                write!(f, "{{ ")?;

                for expr in exprs {
                    write!(f, "{} ", expr.debug(source))?;
                }

                write!(f, "}}")
            }
            RawExpression::List(exprs) => {
                write!(f, "[ ")?;

                for expr in exprs {
                    write!(f, "{} ", expr.debug(source))?;
                }

                write!(f, "]")
            }
            RawExpression::Path(p) => write!(f, "{}", p.debug(source)),
            RawExpression::Boolean(true) => write!(f, "$yes"),
            RawExpression::Boolean(false) => write!(f, "$no"),
        }
    }
}

impl From<Tagged<Path>> for Expression {
    fn from(path: Tagged<Path>) -> Expression {
        path.map(|p| RawExpression::Path(Box::new(p)))
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum Literal {
    Number(Number),
    Size(Number, Unit),
    String(Span),
    Bare,
}

impl ToDebug for Tagged<&Literal> {
    fn fmt_debug(&self, f: &mut fmt::Formatter, source: &str) -> fmt::Result {
        match self.item() {
            Literal::Number(number) => write!(f, "{:?}", *number),
            Literal::Size(number, unit) => write!(f, "{:?}{:?}", *number, unit),
            Literal::String(span) => write!(f, "{}", span.slice(source)),
            Literal::Bare => write!(f, "{}", self.span().slice(source)),
        }
    }
}

impl Literal {
    fn type_name(&self) -> &'static str {
        match self {
            Literal::Number(..) => "number",
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
