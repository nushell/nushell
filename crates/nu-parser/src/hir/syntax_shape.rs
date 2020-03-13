#![allow(clippy::large_enum_variant, clippy::type_complexity)]

mod block;
mod expression;
pub mod flat_shape;

use crate::commands::classified::internal::InternalCommand;
use crate::commands::classified::{ClassifiedCommand, ClassifiedPipeline};
use crate::commands::external_command;
use crate::hir;
use crate::hir::syntax_shape::block::CoerceBlockShape;
use crate::hir::syntax_shape::expression::range::RangeShape;
use crate::hir::syntax_shape::flat_shape::ShapeResult;
use crate::hir::tokens_iterator::TokensIterator;
use crate::hir::{Expression, SpannedExpression};
use crate::parse::operator::EvaluationOperator;
use crate::parse::token_tree::{
    ExternalCommandType, PipelineType, SpannedToken, Token, WhitespaceType, WordType,
};
use crate::parse_command::parse_command_tail;
use derive_new::new;
use getset::Getters;
use nu_errors::ParseError;
use nu_protocol::{ShellTypeName, Signature, SpannedTypeName};
use nu_source::{
    b, DebugDocBuilder, HasSpan, PrettyDebug, PrettyDebugWithSource, Span, Spanned, SpannedItem,
    Tag, TaggedItem, Text,
};
use std::path::{Path, PathBuf};

pub(crate) use self::expression::delimited::DelimitedSquareShape;
pub(crate) use self::expression::file_path::{ExternalWordShape, FilePathShape};
pub(crate) use self::expression::list::{BackoffColoringMode, ExpressionListShape};
pub(crate) use self::expression::number::{
    DecimalShape, IntExpressionShape, IntShape, NumberExpressionShape, NumberShape,
};
pub(crate) use self::expression::pattern::{PatternExpressionShape, PatternShape};
pub(crate) use self::expression::string::{CoerceStringShape, StringExpressionShape, StringShape};
pub(crate) use self::expression::unit::UnitExpressionShape;
pub(crate) use self::expression::variable_path::{
    ColumnPathShape, ColumnPathSyntax, ExpressionContinuationShape, Member, MemberShape,
    PathTailShape, PathTailSyntax, VariablePathShape, VariableShape,
};
pub(crate) use self::expression::{AnyExpressionShape, AnyExpressionStartShape};
pub(crate) use self::flat_shape::FlatShape;

use nu_protocol::SyntaxShape;
use std::fmt::Debug;

impl ExpandSyntax for SyntaxShape {
    type Output = Result<SpannedExpression, ParseError>;

    fn name(&self) -> &'static str {
        match self {
            SyntaxShape::Any => "shape[any]",
            SyntaxShape::Int => "shape[integer]",
            SyntaxShape::Range => "shape[range]",
            SyntaxShape::String => "shape[string]",
            SyntaxShape::Member => "shape[column name]",
            SyntaxShape::ColumnPath => "shape[column path]",
            SyntaxShape::Number => "shape[number]",
            SyntaxShape::Path => "shape[file path]",
            SyntaxShape::Pattern => "shape[glob pattern]",
            SyntaxShape::Block => "shape[block]",
        }
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
    ) -> Result<SpannedExpression, ParseError> {
        match self {
            SyntaxShape::Any => token_nodes.expand_syntax(AnyExpressionShape),
            SyntaxShape::Int => token_nodes
                .expand_syntax(IntExpressionShape)
                .or_else(|_| token_nodes.expand_syntax(VariablePathShape)),
            SyntaxShape::Range => token_nodes
                .expand_syntax(RangeShape)
                .or_else(|_| token_nodes.expand_syntax(VariablePathShape)),
            SyntaxShape::String => token_nodes
                .expand_syntax(CoerceStringShape)
                .or_else(|_| token_nodes.expand_syntax(VariablePathShape)),
            SyntaxShape::Member => {
                let syntax = token_nodes.expand_syntax(MemberShape)?;
                Ok(syntax.to_expr())
            }
            SyntaxShape::ColumnPath => {
                let column_path = token_nodes.expand_syntax(ColumnPathShape)?;
                let ColumnPathSyntax {
                    path: column_path,
                    tag,
                } = column_path;

                Ok(Expression::column_path(column_path).into_expr(tag.span))
            }
            SyntaxShape::Number => token_nodes
                .expand_syntax(NumberExpressionShape)
                .or_else(|_| token_nodes.expand_syntax(VariablePathShape)),
            SyntaxShape::Path => token_nodes
                .expand_syntax(FilePathShape)
                .or_else(|_| token_nodes.expand_syntax(VariablePathShape)),
            SyntaxShape::Pattern => token_nodes
                .expand_syntax(PatternShape)
                .or_else(|_| token_nodes.expand_syntax(VariablePathShape)),
            SyntaxShape::Block => token_nodes
                .expand_syntax(CoerceBlockShape)
                .or_else(|_| token_nodes.expand_syntax(VariablePathShape)),
        }
    }
}

pub trait SignatureRegistry: Debug {
    fn has(&self, name: &str) -> bool;
    fn get(&self, name: &str) -> Option<Signature>;
    fn clone_box(&self) -> Box<dyn SignatureRegistry>;
}

impl SignatureRegistry for Box<dyn SignatureRegistry> {
    fn has(&self, name: &str) -> bool {
        (&**self).has(name)
    }
    fn get(&self, name: &str) -> Option<Signature> {
        (&**self).get(name)
    }
    fn clone_box(&self) -> Box<dyn SignatureRegistry> {
        (&**self).clone_box()
    }
}

#[derive(Debug, Getters, new)]
pub struct ExpandContext<'context> {
    #[get = "pub(crate)"]
    pub registry: Box<dyn SignatureRegistry>,
    pub source: &'context Text,
    pub homedir: Option<PathBuf>,
}

impl<'context> ExpandContext<'context> {
    pub(crate) fn homedir(&self) -> Option<&Path> {
        self.homedir.as_deref()
    }

    pub(crate) fn source(&self) -> &'context Text {
        self.source
    }
}

pub trait ExpandSyntax: std::fmt::Debug + Clone {
    type Output: Clone + std::fmt::Debug + 'static;

    fn name(&self) -> &'static str;

    fn expand<'a, 'b>(&self, token_nodes: &'b mut TokensIterator<'a>) -> Self::Output;
}

pub fn fallible<T, S>(syntax: S) -> FallibleSyntax<S>
where
    T: Clone + Debug + 'static,
    S: ExpandSyntax<Output = T>,
{
    FallibleSyntax { inner: syntax }
}

#[derive(Debug, Copy, Clone)]
pub struct FallibleSyntax<I> {
    inner: I,
}

impl<I, T> ExpandSyntax for FallibleSyntax<I>
where
    I: ExpandSyntax<Output = T>,
    T: Clone + Debug + 'static,
{
    type Output = Result<T, ParseError>;

    fn name(&self) -> &'static str {
        "fallible"
    }

    fn expand<'a, 'b>(&self, token_nodes: &'b mut TokensIterator<'a>) -> Result<T, ParseError> {
        Ok(self.inner.expand(token_nodes))
    }
}

#[derive(Debug, Clone)]
enum BarePathState {
    Initial,
    Seen(Span, Span),
    Error(ParseError),
}

impl BarePathState {
    pub fn seen(self, span: Span) -> BarePathState {
        match self {
            BarePathState::Initial => BarePathState::Seen(span, span),
            BarePathState::Seen(start, _) => BarePathState::Seen(start, span),
            BarePathState::Error(err) => BarePathState::Error(err),
        }
    }

    pub fn end(self, node: Option<&SpannedToken>, expected: &'static str) -> BarePathState {
        match self {
            BarePathState::Initial => match node {
                None => BarePathState::Error(ParseError::unexpected_eof(expected, Span::unknown())),
                Some(token) => {
                    BarePathState::Error(ParseError::mismatch(expected, token.spanned_type_name()))
                }
            },
            BarePathState::Seen(start, end) => BarePathState::Seen(start, end),
            BarePathState::Error(err) => BarePathState::Error(err),
        }
    }

    pub fn into_bare(self) -> Result<Span, ParseError> {
        match self {
            BarePathState::Initial => unreachable!("into_bare in initial state"),
            BarePathState::Seen(start, end) => Ok(start.until(end)),
            BarePathState::Error(err) => Err(err),
        }
    }
}

pub fn expand_bare(
    token_nodes: &'_ mut TokensIterator<'_>,
    predicate: impl Fn(&SpannedToken) -> bool,
) -> Result<Span, ParseError> {
    let mut state = BarePathState::Initial;

    loop {
        if token_nodes.at_end() {
            state = state.end(None, "word");
            break;
        }

        let source = token_nodes.source();

        let mut peeked = token_nodes.peek();
        let node = peeked.node;

        match node {
            Some(token) if predicate(token) => {
                peeked.commit();
                state = state.seen(token.span());
                let shapes = FlatShape::shapes(token, &source);
                token_nodes.color_shapes(shapes);
            }
            token => {
                state = state.end(token, "word");
                break;
            }
        }
    }

    state.into_bare()
}

#[derive(Debug, Copy, Clone)]
pub struct BareExpressionShape;

impl ExpandSyntax for BareExpressionShape {
    type Output = Result<SpannedExpression, ParseError>;

    fn name(&self) -> &'static str {
        "bare expression"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
    ) -> Result<SpannedExpression, ParseError> {
        token_nodes
            .expand_syntax(BarePathShape)
            .map(|span| Expression::bare().into_expr(span))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct BarePathShape;

impl ExpandSyntax for BarePathShape {
    type Output = Result<Span, ParseError>;

    fn name(&self) -> &'static str {
        "bare path"
    }

    fn expand<'a, 'b>(&self, token_nodes: &'b mut TokensIterator<'a>) -> Result<Span, ParseError> {
        expand_bare(token_nodes, |token| match token.unspanned() {
            Token::Bare | Token::EvaluationOperator(EvaluationOperator::Dot) => true,

            _ => false,
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub struct BareShape;

#[derive(Debug, Clone)]
pub struct BareSyntax {
    pub word: String,
    pub span: Span,
}

impl HasSpan for BareSyntax {
    fn span(&self) -> Span {
        self.span
    }
}

impl PrettyDebug for BareSyntax {
    fn pretty(&self) -> DebugDocBuilder {
        b::primitive(&self.word)
    }
}

impl ExpandSyntax for BareShape {
    type Output = Result<BareSyntax, ParseError>;

    fn name(&self) -> &'static str {
        "word"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
    ) -> Result<BareSyntax, ParseError> {
        let source = token_nodes.source();

        token_nodes.expand_token(WordType, |span| {
            Ok((
                FlatShape::Word,
                BareSyntax {
                    word: span.string(&source),
                    span,
                },
            ))
        })
    }
}

#[derive(Debug, Clone)]
pub enum CommandSignature {
    Internal(Spanned<Signature>),
    LiteralExternal { outer: Span, inner: Span },
    External(Span),
    Expression(hir::SpannedExpression),
}

impl PrettyDebugWithSource for CommandSignature {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        match self {
            CommandSignature::Internal(internal) => {
                b::typed("command", b::description(&internal.name))
            }
            CommandSignature::LiteralExternal { outer, .. } => {
                b::typed("command", b::description(outer.slice(source)))
            }
            CommandSignature::External(external) => b::typed(
                "command",
                b::description("^") + b::description(external.slice(source)),
            ),
            CommandSignature::Expression(expr) => b::typed("command", expr.pretty_debug(source)),
        }
    }
}

impl HasSpan for CommandSignature {
    fn span(&self) -> Span {
        match self {
            CommandSignature::Internal(spanned) => spanned.span,
            CommandSignature::LiteralExternal { outer, .. } => *outer,
            CommandSignature::External(span) => *span,
            CommandSignature::Expression(expr) => expr.span,
        }
    }
}

impl CommandSignature {
    pub fn to_expression(&self) -> hir::SpannedExpression {
        match self {
            CommandSignature::Internal(command) => {
                let span = command.span;
                hir::Expression::Command(span).into_expr(span)
            }
            CommandSignature::LiteralExternal { outer, inner } => {
                hir::Expression::ExternalCommand(hir::ExternalCommand::new(*inner))
                    .into_expr(*outer)
            }
            CommandSignature::External(span) => {
                hir::Expression::ExternalCommand(hir::ExternalCommand::new(*span)).into_expr(*span)
            }
            CommandSignature::Expression(expr) => expr.clone(),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct PipelineShape;

impl ExpandSyntax for PipelineShape {
    type Output = ClassifiedPipeline;

    fn name(&self) -> &'static str {
        "pipeline"
    }

    fn expand<'content, 'me>(
        &self,
        token_nodes: &'me mut TokensIterator<'content>,
    ) -> ClassifiedPipeline {
        if token_nodes.at_end() {
            return ClassifiedPipeline::commands(vec![], Span::unknown());
        }

        let start = token_nodes.span_at_cursor();

        // whitespace is allowed at the beginning
        token_nodes.expand_infallible(MaybeSpaceShape);

        let pipeline = token_nodes
            .expand_token(PipelineType, |pipeline| Ok(((), pipeline)))
            .expect("PipelineShape is only expected to be called with a Pipeline token");

        let parts = &pipeline.parts[..];

        let mut out = vec![];

        for part in parts {
            if let Some(span) = part.pipe {
                token_nodes.color_shape(FlatShape::Pipe.spanned(span));
            }

            let tokens: Spanned<&[SpannedToken]> = part.tokens().spanned(part.span());

            let (shapes, classified) = token_nodes.child(tokens, move |token_nodes| {
                token_nodes.expand_infallible(ClassifiedCommandShape)
            });

            for shape in shapes {
                match shape {
                    ShapeResult::Success(shape) => token_nodes.color_shape(shape),
                    ShapeResult::Fallback { shape, allowed } => {
                        token_nodes.color_err(shape, allowed)
                    }
                }
            }

            out.push(classified);
        }

        token_nodes.expand_infallible(BackoffColoringMode::new(vec!["no more tokens".to_string()]));

        let end = token_nodes.span_at_cursor();

        ClassifiedPipeline::commands(out, start.until(end))
    }
}

pub enum CommandHeadKind {
    External,
    Internal(Signature),
}

#[derive(Debug, Copy, Clone)]
pub struct CommandHeadShape;

impl ExpandSyntax for CommandHeadShape {
    type Output = Result<CommandSignature, ParseError>;

    fn name(&self) -> &'static str {
        "command head"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
    ) -> Result<CommandSignature, ParseError> {
        token_nodes.expand_infallible(MaybeSpaceShape);

        let source = token_nodes.source();
        let registry = &token_nodes.context().registry.clone_box();

        token_nodes
            .expand_token(ExternalCommandType, |(inner, outer)| {
                Ok((
                    FlatShape::ExternalCommand,
                    CommandSignature::LiteralExternal { outer, inner },
                ))
            })
            .or_else(|_| {
                token_nodes.expand_token(WordType, |span| {
                    let name = span.slice(&source);
                    if registry.has(name) {
                        let signature = registry.get(name).unwrap();
                        Ok((
                            FlatShape::InternalCommand,
                            CommandSignature::Internal(signature.spanned(span)),
                        ))
                    } else {
                        Ok((FlatShape::ExternalCommand, CommandSignature::External(span)))
                    }
                })
            })
            .or_else(|_| {
                token_nodes
                    .expand_syntax(AnyExpressionShape)
                    .map(CommandSignature::Expression)
            })
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ClassifiedCommandShape;

impl ExpandSyntax for ClassifiedCommandShape {
    type Output = ClassifiedCommand;

    fn name(&self) -> &'static str {
        "classified command"
    }

    fn expand<'a, 'b>(&self, token_nodes: &'b mut TokensIterator<'a>) -> ClassifiedCommand {
        let start = token_nodes.span_at_cursor();
        let source = token_nodes.source();

        let head = match token_nodes.expand_syntax(CommandHeadShape) {
            Err(err) => {
                token_nodes
                    .expand_infallible(BackoffColoringMode::new(vec!["command".to_string()]));
                return ClassifiedCommand::Error(err);
            }

            Ok(head) => head,
        };

        match head {
            CommandSignature::Expression(expr) => ClassifiedCommand::Error(ParseError::mismatch(
                "command",
                expr.type_name().spanned(expr.span),
            )),

            CommandSignature::External(name) => {
                let name_str = name.slice(&source);

                match external_command(token_nodes, name_str.tagged(name)) {
                    Err(err) => ClassifiedCommand::Error(err),
                    Ok(command) => command,
                }
            }

            // If the command starts with `^`, treat it as an external command no matter what
            CommandSignature::LiteralExternal { outer, inner } => {
                let name_str = inner.slice(&source);

                match external_command(token_nodes, name_str.tagged(outer)) {
                    Err(err) => ClassifiedCommand::Error(err),
                    Ok(command) => command,
                }
            }

            CommandSignature::Internal(signature) => {
                let tail = parse_command_tail(&signature.item, token_nodes, signature.span);

                let tail = match tail {
                    Err(err) => {
                        return ClassifiedCommand::Error(err);
                    }
                    Ok(tail) => tail,
                };

                let (positional, named) = match tail {
                    None => (None, None),
                    Some((positional, named)) => (positional, named),
                };

                let end = token_nodes.span_at_cursor();

                let expr = hir::Expression::Command(signature.span).into_expr(signature.span);

                let call = hir::Call {
                    head: Box::new(expr),
                    positional,
                    named,
                    span: start.until(end),
                };

                ClassifiedCommand::Internal(InternalCommand::new(
                    signature.item.name.clone(),
                    Tag {
                        span: signature.span,
                        anchor: None,
                    },
                    call,
                ))
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct MaybeWhitespaceEof;

impl ExpandSyntax for MaybeWhitespaceEof {
    type Output = Result<(), ParseError>;

    fn name(&self) -> &'static str {
        "<whitespace? eof>"
    }

    fn expand<'a, 'b>(&self, token_nodes: &'b mut TokensIterator<'a>) -> Self::Output {
        token_nodes.atomic_parse(|token_nodes| {
            token_nodes.expand_infallible(MaybeSpaceShape);
            token_nodes.expand_syntax(EofShape)
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub struct EofShape;

impl ExpandSyntax for EofShape {
    type Output = Result<(), ParseError>;

    fn name(&self) -> &'static str {
        "eof"
    }

    fn expand<'a, 'b>(&self, token_nodes: &'b mut TokensIterator<'a>) -> Result<(), ParseError> {
        let next = token_nodes.peek();
        let node = next.node;

        match node {
            None => Ok(()),
            Some(node) => Err(ParseError::mismatch("eof", node.spanned_type_name())),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct WhitespaceShape;

impl ExpandSyntax for WhitespaceShape {
    type Output = Result<Span, ParseError>;

    fn name(&self) -> &'static str {
        "whitespace"
    }

    fn expand<'a, 'b>(&self, token_nodes: &'b mut TokensIterator<'a>) -> Result<Span, ParseError> {
        token_nodes.expand_token(WhitespaceType, |span| Ok((FlatShape::Whitespace, span)))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct MaybeSpaceShape;

impl ExpandSyntax for MaybeSpaceShape {
    type Output = Option<Span>;

    fn name(&self) -> &'static str {
        "whitespace?"
    }

    fn expand<'a, 'b>(&self, token_nodes: &'b mut TokensIterator<'a>) -> Option<Span> {
        let result = token_nodes.expand_token(WhitespaceType, |span| {
            Ok((FlatShape::Whitespace, Some(span)))
        });

        // No space is acceptable, but we need to err inside expand_token so we don't
        // consume the non-whitespace token
        result.unwrap_or(None)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct SpaceShape;

#[derive(Debug, Copy, Clone)]
pub struct CommandShape;
