pub(crate) mod baseline_parse;
pub(crate) mod binary;
pub(crate) mod expand_external_tokens;
pub(crate) mod external_command;
pub(crate) mod named;
pub(crate) mod path;
pub(crate) mod syntax_shape;
pub(crate) mod tokens_iterator;

use crate::parser::hir::path::PathMember;
use crate::parser::hir::syntax_shape::Member;
use crate::parser::{registry, Operator, Unit};
use crate::prelude::*;
use derive_new::new;
use getset::Getters;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

use crate::evaluate::Scope;
use crate::parser::parse::tokens::RawNumber;
use crate::traits::ToDebug;

pub(crate) use self::binary::Binary;
pub(crate) use self::external_command::ExternalCommand;
pub(crate) use self::named::NamedArguments;
pub(crate) use self::path::Path;
pub(crate) use self::syntax_shape::ExpandContext;
pub(crate) use self::tokens_iterator::TokensIterator;

pub use self::syntax_shape::SyntaxShape;

#[derive(Debug, Clone, Eq, PartialEq, Getters, Serialize, Deserialize, new)]
pub struct Call {
    #[get = "pub(crate)"]
    pub head: Box<Expression>,
    #[get = "pub(crate)"]
    pub positional: Option<Vec<Expression>>,
    #[get = "pub(crate)"]
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

impl FormatDebug for Call {
    fn fmt_debug(&self, f: &mut DebugFormatter, source: &str) -> fmt::Result {
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
    ExternalWord,
    Synthetic(Synthetic),
    Variable(Variable),
    Binary(Box<Binary>),
    Block(Vec<Expression>),
    List(Vec<Expression>),
    Path(Box<Path>),

    FilePath(PathBuf),
    ExternalCommand(ExternalCommand),
    Command(Span),

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
            RawExpression::Command(..) => "command",
            RawExpression::ExternalWord => "external word",
            RawExpression::FilePath(..) => "file path",
            RawExpression::Variable(..) => "variable",
            RawExpression::List(..) => "list",
            RawExpression::Binary(..) => "binary",
            RawExpression::Block(..) => "block",
            RawExpression::Path(..) => "variable path",
            RawExpression::Boolean(..) => "boolean",
            RawExpression::ExternalCommand(..) => "external",
        }
    }
}

pub type Expression = Spanned<RawExpression>;

impl std::fmt::Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let span = self.span;

        match &self.item {
            RawExpression::Literal(literal) => write!(f, "{}", literal.tagged(self.span)),
            RawExpression::Synthetic(Synthetic::String(s)) => write!(f, "{}", s),
            RawExpression::Command(_) => write!(f, "Command{{ {}..{} }}", span.start(), span.end()),
            RawExpression::ExternalWord => {
                write!(f, "ExternalWord{{ {}..{} }}", span.start(), span.end())
            }
            RawExpression::FilePath(file) => write!(f, "Path{{ {} }}", file.display()),
            RawExpression::Variable(variable) => write!(f, "{}", variable),
            RawExpression::List(list) => f
                .debug_list()
                .entries(list.iter().map(|e| format!("{}", e)))
                .finish(),
            RawExpression::Binary(binary) => write!(f, "{}", binary),
            RawExpression::Block(items) => {
                write!(f, "Block")?;
                f.debug_set()
                    .entries(items.iter().map(|i| format!("{}", i)))
                    .finish()
            }
            RawExpression::Path(path) => write!(f, "{}", path),
            RawExpression::Boolean(b) => write!(f, "${}", b),
            RawExpression::ExternalCommand(..) => {
                write!(f, "ExternalComment{{ {}..{} }}", span.start(), span.end())
            }
        }
    }
}

impl Expression {
    pub(crate) fn number(i: impl Into<Number>, span: impl Into<Span>) -> Expression {
        RawExpression::Literal(Literal::Number(i.into())).spanned(span.into())
    }

    pub(crate) fn size(
        i: impl Into<Number>,
        unit: impl Into<Unit>,
        span: impl Into<Span>,
    ) -> Expression {
        RawExpression::Literal(Literal::Size(i.into(), unit.into())).spanned(span.into())
    }

    pub(crate) fn synthetic_string(s: impl Into<String>) -> Expression {
        RawExpression::Synthetic(Synthetic::String(s.into())).spanned_unknown()
    }

    pub(crate) fn string(inner: impl Into<Span>, outer: impl Into<Span>) -> Expression {
        RawExpression::Literal(Literal::String(inner.into())).spanned(outer.into())
    }

    pub(crate) fn column_path(members: Vec<Member>, span: impl Into<Span>) -> Expression {
        RawExpression::Literal(Literal::ColumnPath(members)).spanned(span.into())
    }

    pub(crate) fn path(
        head: Expression,
        tail: Vec<impl Into<PathMember>>,
        span: impl Into<Span>,
    ) -> Expression {
        let tail = tail.into_iter().map(|t| t.into()).collect();
        RawExpression::Path(Box::new(Path::new(head, tail))).spanned(span.into())
    }

    pub(crate) fn dot_member(head: Expression, next: impl Into<PathMember>) -> Expression {
        let Spanned { item, span } = head;
        let next = next.into();

        let new_span = head.span.until(next.span);

        match item {
            RawExpression::Path(path) => {
                let (head, mut tail) = path.parts();

                tail.push(next);
                Expression::path(head, tail, new_span)
            }

            other => Expression::path(other.spanned(span), vec![next], new_span),
        }
    }

    pub(crate) fn infix(
        left: Expression,
        op: Spanned<impl Into<Operator>>,
        right: Expression,
    ) -> Expression {
        let new_span = left.span.until(right.span);

        RawExpression::Binary(Box::new(Binary::new(left, op.map(|o| o.into()), right)))
            .spanned(new_span)
    }

    pub(crate) fn file_path(path: impl Into<PathBuf>, outer: impl Into<Span>) -> Expression {
        RawExpression::FilePath(path.into()).spanned(outer)
    }

    pub(crate) fn list(list: Vec<Expression>, span: impl Into<Span>) -> Expression {
        RawExpression::List(list).spanned(span)
    }

    pub(crate) fn bare(span: impl Into<Span>) -> Expression {
        RawExpression::Literal(Literal::Bare).spanned(span)
    }

    pub(crate) fn pattern(inner: impl Into<String>, outer: impl Into<Span>) -> Expression {
        RawExpression::Literal(Literal::GlobPattern(inner.into())).spanned(outer.into())
    }

    pub(crate) fn variable(inner: impl Into<Span>, outer: impl Into<Span>) -> Expression {
        RawExpression::Variable(Variable::Other(inner.into())).spanned(outer)
    }

    pub(crate) fn external_command(inner: impl Into<Span>, outer: impl Into<Span>) -> Expression {
        RawExpression::ExternalCommand(ExternalCommand::new(inner.into())).spanned(outer)
    }

    pub(crate) fn it_variable(inner: impl Into<Span>, outer: impl Into<Span>) -> Expression {
        RawExpression::Variable(Variable::It(inner.into())).spanned(outer)
    }
}

impl FormatDebug for Spanned<RawExpression> {
    fn fmt_debug(&self, f: &mut DebugFormatter, source: &str) -> fmt::Result {
        match &self.item {
            RawExpression::Literal(l) => l.spanned(self.span).fmt_debug(f, source),
            RawExpression::FilePath(p) => write!(f, "{}", p.display()),
            RawExpression::ExternalWord => write!(f, "{}", self.span.slice(source)),
            RawExpression::Command(tag) => write!(f, "{}", tag.slice(source)),
            RawExpression::Synthetic(Synthetic::String(s)) => write!(f, "{:?}", s),
            RawExpression::Variable(Variable::It(_)) => write!(f, "$it"),
            RawExpression::Variable(Variable::Other(s)) => write!(f, "${}", s.slice(source)),
            RawExpression::Binary(b) => write!(f, "{}", b.debug(source)),
            RawExpression::ExternalCommand(c) => write!(f, "^{}", c.name().slice(source)),
            RawExpression::Block(exprs) => f.say_block("block", |f| {
                write!(f, "{{ ")?;

                for expr in exprs {
                    write!(f, "{} ", expr.debug(source))?;
                }

                write!(f, "}}")
            }),
            RawExpression::List(exprs) => f.say_block("list", |f| {
                write!(f, "[ ")?;

                for expr in exprs {
                    write!(f, "{} ", expr.debug(source))?;
                }

                write!(f, "]")
            }),
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

/// Literals are expressions that are:
///
/// 1. Copy
/// 2. Can be evaluated without additional context
/// 3. Evaluation cannot produce an error
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum Literal {
    Number(Number),
    Size(Number, Unit),
    String(Span),
    GlobPattern(String),
    ColumnPath(Vec<Member>),
    Bare,
}

impl std::fmt::Display for Tagged<Literal> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Tagged::new(self.tag.clone(), &self.item))
    }
}

impl std::fmt::Display for Tagged<&Literal> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let span = self.tag.span;

        match &self.item {
            Literal::Number(number) => write!(f, "{}", number),
            Literal::Size(number, unit) => write!(f, "{}{}", number, unit.as_str()),
            Literal::String(_) => write!(f, "String{{ {}..{} }}", span.start(), span.end()),
            Literal::ColumnPath(_) => write!(f, "ColumnPath"),
            Literal::GlobPattern(_) => write!(f, "Glob{{ {}..{} }}", span.start(), span.end()),
            Literal::Bare => write!(f, "Bare{{ {}..{} }}", span.start(), span.end()),
        }
    }
}

impl FormatDebug for Spanned<&Literal> {
    fn fmt_debug(&self, f: &mut DebugFormatter, source: &str) -> fmt::Result {
        match self.item {
            Literal::Number(..) => f.say_str("number", self.span.slice(source)),
            Literal::Size(..) => f.say_str("size", self.span.slice(source)),
            Literal::String(..) => f.say_str("string", self.span.slice(source)),
            Literal::ColumnPath(path) => f.say_block("column path", |f| {
                write!(f, "[ ")?;

                for member in path {
                    write!(f, "{} ", member.debug(source))?;
                }

                write!(f, "]")
            }),
            Literal::GlobPattern(..) => f.say_str("glob", self.span.slice(source)),
            Literal::Bare => f.say_str("word", self.span.slice(source)),
        }
    }
}

impl Literal {
    fn type_name(&self) -> &'static str {
        match self {
            Literal::Number(..) => "number",
            Literal::Size(..) => "size",
            Literal::String(..) => "string",
            Literal::ColumnPath(..) => "column path",
            Literal::Bare => "string",
            Literal::GlobPattern(_) => "pattern",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum Variable {
    It(Span),
    Other(Span),
}

impl std::fmt::Display for Variable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Variable::It(_) => write!(f, "$it"),
            Variable::Other(span) => write!(f, "${{ {}..{} }}", span.start(), span.end()),
        }
    }
}

impl FormatDebug for Spanned<Variable> {
    fn fmt_debug(&self, f: &mut DebugFormatter, source: &str) -> fmt::Result {
        write!(f, "{}", self.span.slice(source))
    }
}
