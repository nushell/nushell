pub(crate) mod baseline_parse;
pub(crate) mod binary;
pub(crate) mod expand_external_tokens;
pub(crate) mod external_command;
pub(crate) mod named;
pub(crate) mod path;
pub(crate) mod range;
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
    b, DebugDocBuilder, HasSpan, IntoSpanned, PrettyDebug, PrettyDebugRefineKind,
    PrettyDebugWithSource, Span, Spanned,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::parse::number::RawNumber;

pub(crate) use self::binary::Binary;
pub(crate) use self::path::Path;
pub(crate) use self::range::Range;
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
    pub head: Box<SpannedExpression>,
    #[get = "pub(crate)"]
    pub positional: Option<Vec<SpannedExpression>>,
    #[get = "pub(crate)"]
    pub named: Option<NamedArguments>,
    pub span: Span,
}

impl Call {
    pub fn switch_preset(&self, switch: &str) -> bool {
        self.named
            .as_ref()
            .map(|n| n.switch_present(switch))
            .unwrap_or(false)
    }
}

impl PrettyDebugWithSource for Call {
    fn refined_pretty_debug(&self, refine: PrettyDebugRefineKind, source: &str) -> DebugDocBuilder {
        match refine {
            PrettyDebugRefineKind::ContextFree => self.pretty_debug(source),
            PrettyDebugRefineKind::WithContext => {
                self.head
                    .refined_pretty_debug(PrettyDebugRefineKind::WithContext, source)
                    + b::preceded_option(
                        Some(b::space()),
                        self.positional.as_ref().map(|pos| {
                            b::intersperse(
                                pos.iter().map(|expr| {
                                    expr.refined_pretty_debug(
                                        PrettyDebugRefineKind::WithContext,
                                        source,
                                    )
                                }),
                                b::space(),
                            )
                        }),
                    )
                    + b::preceded_option(
                        Some(b::space()),
                        self.named.as_ref().map(|named| {
                            named.refined_pretty_debug(PrettyDebugRefineKind::WithContext, source)
                        }),
                    )
            }
        }
    }

    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        b::typed(
            "call",
            self.refined_pretty_debug(PrettyDebugRefineKind::WithContext, source),
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
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
}

impl ShellTypeName for Expression {
    fn type_name(&self) -> &'static str {
        match self {
            Expression::Literal(literal) => literal.type_name(),
            Expression::Synthetic(synthetic) => synthetic.type_name(),
            Expression::Command(..) => "command",
            Expression::ExternalWord => "external word",
            Expression::FilePath(..) => "file path",
            Expression::Variable(..) => "variable",
            Expression::List(..) => "list",
            Expression::Binary(..) => "binary",
            Expression::Range(..) => "range",
            Expression::Block(..) => "block",
            Expression::Path(..) => "variable path",
            Expression::Boolean(..) => "boolean",
            Expression::ExternalCommand(..) => "external",
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

impl IntoSpanned for Expression {
    type Output = SpannedExpression;

    fn into_spanned(self, span: impl Into<Span>) -> Self::Output {
        SpannedExpression {
            expr: self,
            span: span.into(),
        }
    }
}

impl Expression {
    pub fn into_expr(self, span: impl Into<Span>) -> SpannedExpression {
        self.into_spanned(span)
    }

    pub fn into_unspanned_expr(self) -> SpannedExpression {
        SpannedExpression {
            expr: self,
            span: Span::unknown(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct SpannedExpression {
    pub expr: Expression,
    pub span: Span,
}

impl SpannedExpression {
    pub fn new(expr: Expression, span: Span) -> SpannedExpression {
        SpannedExpression { expr, span }
    }
}

impl std::ops::Deref for SpannedExpression {
    type Target = Expression;

    fn deref(&self) -> &Expression {
        &self.expr
    }
}

impl HasSpan for SpannedExpression {
    fn span(&self) -> Span {
        self.span
    }
}

impl ShellTypeName for SpannedExpression {
    fn type_name(&self) -> &'static str {
        self.expr.type_name()
    }
}

impl PrettyDebugWithSource for SpannedExpression {
    fn refined_pretty_debug(&self, refine: PrettyDebugRefineKind, source: &str) -> DebugDocBuilder {
        match refine {
            PrettyDebugRefineKind::ContextFree => self.refined_pretty_debug(refine, source),
            PrettyDebugRefineKind::WithContext => match &self.expr {
                Expression::Literal(literal) => literal
                    .clone()
                    .into_spanned(self.span)
                    .refined_pretty_debug(refine, source),
                Expression::ExternalWord => {
                    b::delimit("e\"", b::primitive(self.span.slice(source)), "\"").group()
                }
                Expression::Synthetic(s) => match s {
                    Synthetic::String(_) => {
                        b::delimit("s\"", b::primitive(self.span.slice(source)), "\"").group()
                    }
                },
                Expression::Variable(Variable::Other(_)) => b::keyword(self.span.slice(source)),
                Expression::Variable(Variable::It(_)) => b::keyword("$it"),
                Expression::Binary(binary) => binary.pretty_debug(source),
                Expression::Range(range) => range.pretty_debug(source),
                Expression::Block(_) => b::opaque("block"),
                Expression::List(list) => b::delimit(
                    "[",
                    b::intersperse(
                        list.iter()
                            .map(|item| item.refined_pretty_debug(refine, source)),
                        b::space(),
                    ),
                    "]",
                ),
                Expression::Path(path) => path.pretty_debug(source),
                Expression::FilePath(path) => b::typed("path", b::primitive(path.display())),
                Expression::ExternalCommand(external) => {
                    b::keyword("^") + b::keyword(external.name.slice(source))
                }
                Expression::Command(command) => b::keyword(command.slice(source)),
                Expression::Boolean(boolean) => match boolean {
                    true => b::primitive("$yes"),
                    false => b::primitive("$no"),
                },
            },
        }
    }

    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        match &self.expr {
            Expression::Literal(literal) => {
                literal.clone().into_spanned(self.span).pretty_debug(source)
            }
            Expression::ExternalWord => {
                b::typed("external word", b::primitive(self.span.slice(source)))
            }
            Expression::Synthetic(s) => match s {
                Synthetic::String(s) => b::typed("synthetic", b::primitive(format!("{:?}", s))),
            },
            Expression::Variable(Variable::Other(_)) => b::keyword(self.span.slice(source)),
            Expression::Variable(Variable::It(_)) => b::keyword("$it"),
            Expression::Binary(binary) => binary.pretty_debug(source),
            Expression::Range(range) => range.pretty_debug(source),
            Expression::Block(_) => b::opaque("block"),
            Expression::List(list) => b::delimit(
                "[",
                b::intersperse(
                    list.iter().map(|item| item.pretty_debug(source)),
                    b::space(),
                ),
                "]",
            ),
            Expression::Path(path) => path.pretty_debug(source),
            Expression::FilePath(path) => b::typed("path", b::primitive(path.display())),
            Expression::ExternalCommand(external) => b::typed(
                "command",
                b::keyword("^") + b::primitive(external.name.slice(source)),
            ),
            Expression::Command(command) => {
                b::typed("command", b::primitive(command.slice(source)))
            }
            Expression::Boolean(boolean) => match boolean {
                true => b::primitive("$yes"),
                false => b::primitive("$no"),
            },
        }
    }
}

impl Expression {
    pub fn number(i: impl Into<Number>) -> Expression {
        Expression::Literal(Literal::Number(i.into()))
    }

    pub fn size(i: impl Into<Number>, unit: impl Into<Unit>) -> Expression {
        Expression::Literal(Literal::Size(i.into(), unit.into()))
    }

    pub fn string(inner: impl Into<Span>) -> Expression {
        Expression::Literal(Literal::String(inner.into()))
    }

    pub fn synthetic_string(string: impl Into<String>) -> Expression {
        Expression::Synthetic(Synthetic::String(string.into()))
    }

    pub fn column_path(members: Vec<Member>) -> Expression {
        Expression::Literal(Literal::ColumnPath(members))
    }

    pub fn path(head: SpannedExpression, tail: Vec<impl Into<PathMember>>) -> Expression {
        let tail = tail.into_iter().map(|t| t.into()).collect();
        Expression::Path(Box::new(Path::new(head, tail)))
    }

    pub fn dot_member(head: SpannedExpression, next: impl Into<PathMember>) -> Expression {
        let SpannedExpression { expr: item, span } = head;
        let next = next.into();

        match item {
            Expression::Path(path) => {
                let (head, mut tail) = path.parts();

                tail.push(next);
                Expression::path(head, tail)
            }

            other => Expression::path(other.into_expr(span), vec![next]),
        }
    }

    pub fn infix(
        left: SpannedExpression,
        op: Spanned<impl Into<CompareOperator>>,
        right: SpannedExpression,
    ) -> Expression {
        Expression::Binary(Box::new(Binary::new(left, op.map(|o| o.into()), right)))
    }

    pub fn range(left: SpannedExpression, op: Span, right: SpannedExpression) -> Expression {
        Expression::Range(Box::new(Range::new(left, op, right)))
    }

    pub fn file_path(path: impl Into<PathBuf>) -> Expression {
        Expression::FilePath(path.into())
    }

    pub fn list(list: Vec<SpannedExpression>) -> Expression {
        Expression::List(list)
    }

    pub fn bare() -> Expression {
        Expression::Literal(Literal::Bare)
    }

    pub fn pattern(inner: impl Into<String>) -> Expression {
        Expression::Literal(Literal::GlobPattern(inner.into()))
    }

    pub fn variable(inner: impl Into<Span>) -> Expression {
        Expression::Variable(Variable::Other(inner.into()))
    }

    pub fn external_command(inner: impl Into<Span>) -> Expression {
        Expression::ExternalCommand(ExternalCommand::new(inner.into()))
    }

    pub fn it_variable(inner: impl Into<Span>) -> Expression {
        Expression::Variable(Variable::It(inner.into()))
    }
}

impl From<Spanned<Path>> for SpannedExpression {
    fn from(path: Spanned<Path>) -> SpannedExpression {
        Expression::Path(Box::new(path.item)).into_expr(path.span)
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

impl Literal {
    pub fn into_spanned(self, span: impl Into<Span>) -> SpannedLiteral {
        SpannedLiteral {
            literal: self,
            span: span.into(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct SpannedLiteral {
    pub literal: Literal,
    pub span: Span,
}

impl ShellTypeName for Literal {
    fn type_name(&self) -> &'static str {
        match &self {
            Literal::Number(..) => "number",
            Literal::Size(..) => "size",
            Literal::String(..) => "string",
            Literal::ColumnPath(..) => "column path",
            Literal::Bare => "string",
            Literal::GlobPattern(_) => "pattern",
        }
    }
}

impl PrettyDebugWithSource for SpannedLiteral {
    fn refined_pretty_debug(&self, refine: PrettyDebugRefineKind, source: &str) -> DebugDocBuilder {
        match refine {
            PrettyDebugRefineKind::ContextFree => self.pretty_debug(source),
            PrettyDebugRefineKind::WithContext => match &self.literal {
                Literal::Number(number) => number.pretty(),
                Literal::Size(number, unit) => (number.pretty() + unit.pretty()).group(),
                Literal::String(string) => b::primitive(format!("{:?}", string.slice(source))),
                Literal::GlobPattern(pattern) => b::primitive(pattern),
                Literal::ColumnPath(path) => {
                    b::intersperse_with_source(path.iter(), b::space(), source)
                }
                Literal::Bare => b::delimit("b\"", b::primitive(self.span.slice(source)), "\""),
            },
        }
    }

    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        match &self.literal {
            Literal::Number(number) => number.pretty(),
            Literal::Size(number, unit) => {
                b::typed("size", (number.pretty() + unit.pretty()).group())
            }
            Literal::String(string) => b::typed(
                "string",
                b::primitive(format!("{:?}", string.slice(source))),
            ),
            Literal::GlobPattern(pattern) => b::typed("pattern", b::primitive(pattern)),
            Literal::ColumnPath(path) => b::typed(
                "column path",
                b::intersperse_with_source(path.iter(), b::space(), source),
            ),
            Literal::Bare => b::typed("bare", b::primitive(self.span.slice(source))),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum Variable {
    It(Span),
    Other(Span),
}
