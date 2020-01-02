mod block;
mod expression;
pub mod flat_shape;

use crate::commands::classified::internal::InternalCommand;
use crate::commands::classified::{ClassifiedCommand, ClassifiedPipeline};
use crate::commands::external_command;
use crate::hir;
use crate::hir::expand_external_tokens::ExternalTokensShape;
use crate::hir::syntax_shape::block::AnyBlockShape;
use crate::hir::syntax_shape::expression::range::RangeShape;
use crate::hir::tokens_iterator::{Peeked, TokensIterator};
use crate::parse::operator::EvaluationOperator;
use crate::parse::token_tree::TokenNode;
use crate::parse::tokens::{Token, UnspannedToken};
use crate::parse_command::{parse_command_tail, CommandTailShape};
use derive_new::new;
use getset::Getters;
use nu_errors::{ParseError, ShellError};
use nu_protocol::{ShellTypeName, Signature};
use nu_source::{
    b, DebugDocBuilder, HasFallibleSpan, HasSpan, PrettyDebug, PrettyDebugWithSource, Span,
    Spanned, SpannedItem, Tag, TaggedItem, Text,
};
use std::path::{Path, PathBuf};

pub(crate) use self::expression::atom::{
    expand_atom, AtomicToken, ExpansionRule, UnspannedAtomicToken,
};
pub(crate) use self::expression::delimited::{
    color_delimited_square, expand_delimited_square, DelimitedShape,
};
pub(crate) use self::expression::file_path::FilePathShape;
pub(crate) use self::expression::list::{BackoffColoringMode, ExpressionListShape};
pub(crate) use self::expression::number::{IntShape, NumberShape};
pub(crate) use self::expression::pattern::{BarePatternShape, PatternShape};
pub(crate) use self::expression::string::StringShape;
pub(crate) use self::expression::unit::{UnitShape, UnitSyntax};
pub(crate) use self::expression::variable_path::{
    ColorableDotShape, ColumnPathShape, ColumnPathSyntax, DotShape, ExpressionContinuation,
    ExpressionContinuationShape, Member, MemberShape, PathTailShape, PathTailSyntax,
    VariablePathShape,
};
pub(crate) use self::expression::{continue_expression, AnyExpressionShape};
pub(crate) use self::flat_shape::FlatShape;

use nu_protocol::SyntaxShape;

impl FallibleColorSyntax for SyntaxShape {
    type Info = ();
    type Input = ();

    fn name(&self) -> &'static str {
        "SyntaxShape"
    }

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<(), ShellError> {
        match self {
            SyntaxShape::Any => color_fallible_syntax(&AnyExpressionShape, token_nodes, context),
            SyntaxShape::Int => color_fallible_syntax(&IntShape, token_nodes, context),
            SyntaxShape::Range => color_fallible_syntax(&RangeShape, token_nodes, context),
            SyntaxShape::String => {
                color_fallible_syntax_with(&StringShape, &FlatShape::String, token_nodes, context)
            }
            SyntaxShape::Member => color_fallible_syntax(&MemberShape, token_nodes, context),
            SyntaxShape::ColumnPath => {
                color_fallible_syntax(&ColumnPathShape, token_nodes, context)
            }
            SyntaxShape::Number => color_fallible_syntax(&NumberShape, token_nodes, context),
            SyntaxShape::Path => color_fallible_syntax(&FilePathShape, token_nodes, context),
            SyntaxShape::Pattern => color_fallible_syntax(&PatternShape, token_nodes, context),
            SyntaxShape::Block => color_fallible_syntax(&AnyBlockShape, token_nodes, context),
        }
    }
}

impl ExpandExpression for SyntaxShape {
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

    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<hir::Expression, ParseError> {
        match self {
            SyntaxShape::Any => expand_expr(&AnyExpressionShape, token_nodes, context),
            SyntaxShape::Int => expand_expr(&IntShape, token_nodes, context),
            SyntaxShape::Range => expand_expr(&RangeShape, token_nodes, context),
            SyntaxShape::String => expand_expr(&StringShape, token_nodes, context),
            SyntaxShape::Member => {
                let syntax = expand_syntax(&MemberShape, token_nodes, context)?;
                Ok(syntax.to_expr())
            }
            SyntaxShape::ColumnPath => {
                let column_path = expand_syntax(&ColumnPathShape, token_nodes, context)?;
                let ColumnPathSyntax {
                    path: column_path,
                    tag,
                } = column_path;

                Ok(hir::Expression::column_path(column_path, tag.span))
            }
            SyntaxShape::Number => expand_expr(&NumberShape, token_nodes, context),
            SyntaxShape::Path => expand_expr(&FilePathShape, token_nodes, context),
            SyntaxShape::Pattern => expand_expr(&PatternShape, token_nodes, context),
            SyntaxShape::Block => expand_expr(&AnyBlockShape, token_nodes, context),
        }
    }
}

pub trait SignatureRegistry {
    fn has(&self, name: &str) -> Result<bool, ShellError>;
    fn get(&self, name: &str) -> Result<Option<Signature>, ShellError>;
}

#[derive(Getters, new)]
pub struct ExpandContext<'context> {
    #[get = "pub(crate)"]
    pub registry: Box<dyn SignatureRegistry>,
    pub source: &'context Text,
    pub homedir: Option<PathBuf>,
}

impl<'context> ExpandContext<'context> {
    pub(crate) fn homedir(&self) -> Option<&Path> {
        self.homedir.as_ref().map(|h| h.as_path())
    }

    pub(crate) fn source(&self) -> &'context Text {
        self.source
    }
}

pub trait TestSyntax: std::fmt::Debug + Copy {
    fn test<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Option<Peeked<'a, 'b>>;
}

pub trait ExpandExpression: std::fmt::Debug + Copy {
    fn name(&self) -> &'static str;

    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<hir::Expression, ParseError>;
}

pub trait FallibleColorSyntax: std::fmt::Debug + Copy {
    type Info;
    type Input;

    fn name(&self) -> &'static str;

    fn color_syntax<'a, 'b>(
        &self,
        input: &Self::Input,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Self::Info, ShellError>;
}

pub trait ColorSyntax: std::fmt::Debug + Copy {
    type Info;
    type Input;

    fn name(&self) -> &'static str;

    fn color_syntax<'a, 'b>(
        &self,
        input: &Self::Input,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Self::Info;
}

pub trait ExpandSyntax: std::fmt::Debug + Copy {
    type Output: HasFallibleSpan + Clone + std::fmt::Debug + 'static;

    fn name(&self) -> &'static str;

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Self::Output, ParseError>;
}

pub fn expand_syntax<'a, 'b, T: ExpandSyntax>(
    shape: &T,
    token_nodes: &'b mut TokensIterator<'a>,
    context: &ExpandContext,
) -> Result<T::Output, ParseError> {
    token_nodes.expand_frame(shape.name(), |token_nodes| {
        shape.expand_syntax(token_nodes, context)
    })
}

pub(crate) fn expand_expr<'a, 'b, T: ExpandExpression>(
    shape: &T,
    token_nodes: &'b mut TokensIterator<'a>,
    context: &ExpandContext,
) -> Result<hir::Expression, ParseError> {
    token_nodes.expand_expr_frame(shape.name(), |token_nodes| {
        shape.expand_expr(token_nodes, context)
    })
}

pub fn color_syntax<'a, 'b, T: ColorSyntax<Info = U, Input = ()>, U>(
    shape: &T,
    token_nodes: &'b mut TokensIterator<'a>,
    context: &ExpandContext,
) -> ((), U) {
    (
        (),
        token_nodes.color_frame(shape.name(), |token_nodes| {
            shape.color_syntax(&(), token_nodes, context)
        }),
    )
}

pub fn color_fallible_syntax<'a, 'b, T: FallibleColorSyntax<Info = U, Input = ()>, U>(
    shape: &T,
    token_nodes: &'b mut TokensIterator<'a>,
    context: &ExpandContext,
) -> Result<U, ShellError> {
    token_nodes.color_fallible_frame(shape.name(), |token_nodes| {
        shape.color_syntax(&(), token_nodes, context)
    })
}

pub fn color_syntax_with<'a, 'b, T: ColorSyntax<Info = U, Input = I>, U, I>(
    shape: &T,
    input: &I,
    token_nodes: &'b mut TokensIterator<'a>,
    context: &ExpandContext,
) -> ((), U) {
    (
        (),
        token_nodes.color_frame(shape.name(), |token_nodes| {
            shape.color_syntax(input, token_nodes, context)
        }),
    )
}

pub fn color_fallible_syntax_with<'a, 'b, T: FallibleColorSyntax<Info = U, Input = I>, U, I>(
    shape: &T,
    input: &I,
    token_nodes: &'b mut TokensIterator<'a>,
    context: &ExpandContext,
) -> Result<U, ShellError> {
    token_nodes.color_fallible_frame(shape.name(), |token_nodes| {
        shape.color_syntax(input, token_nodes, context)
    })
}

impl<T: ExpandExpression> ExpandSyntax for T {
    type Output = hir::Expression;

    fn name(&self) -> &'static str {
        ExpandExpression::name(self)
    }

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Self::Output, ParseError> {
        ExpandExpression::expand_expr(self, token_nodes, context)
    }
}

pub trait SkipSyntax: std::fmt::Debug + Copy {
    fn skip<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<(), ShellError>;
}

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

    pub fn end(self, peeked: Peeked, reason: &'static str) -> BarePathState {
        match self {
            BarePathState::Initial => BarePathState::Error(peeked.type_error(reason)),
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

pub fn expand_bare<'a, 'b>(
    token_nodes: &'b mut TokensIterator<'a>,
    _context: &ExpandContext,
    predicate: impl Fn(&TokenNode) -> bool,
) -> Result<Span, ParseError> {
    let mut state = BarePathState::Initial;

    loop {
        // Whitespace ends a word
        let mut peeked = token_nodes.peek_any();

        match peeked.node {
            None => {
                state = state.end(peeked, "word");
                break;
            }
            Some(node) => {
                if predicate(node) {
                    state = state.seen(node.span());
                    peeked.commit();
                } else {
                    state = state.end(peeked, "word");
                    break;
                }
            }
        }
    }

    state.into_bare()
}

#[derive(Debug, Copy, Clone)]
pub struct BarePathShape;

impl ExpandSyntax for BarePathShape {
    type Output = Span;

    fn name(&self) -> &'static str {
        "bare path"
    }

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Span, ParseError> {
        expand_bare(token_nodes, context, |token| match token {
            TokenNode::Token(Token {
                unspanned: UnspannedToken::Bare,
                ..
            })
            | TokenNode::Token(Token {
                unspanned: UnspannedToken::EvaluationOperator(EvaluationOperator::Dot),
                ..
            }) => true,

            _ => false,
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub struct BareShape;

impl FallibleColorSyntax for BareShape {
    type Info = ();
    type Input = FlatShape;

    fn name(&self) -> &'static str {
        "BareShape"
    }

    fn color_syntax<'a, 'b>(
        &self,
        input: &FlatShape,
        token_nodes: &'b mut TokensIterator<'a>,
        _context: &ExpandContext,
    ) -> Result<(), ShellError> {
        let span = token_nodes.peek_any_token("word", |token| match token {
            // If it's a bare token, color it
            TokenNode::Token(Token { span, .. }) => Ok(span),

            // otherwise, fail
            other => Err(ParseError::mismatch(
                "word",
                other.type_name().spanned(other.span()),
            )),
        })?;

        token_nodes.color_shape((*input).spanned(*span));

        Ok(())
    }
}

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
    type Output = BareSyntax;

    fn name(&self) -> &'static str {
        "word"
    }

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Self::Output, ParseError> {
        let peeked = token_nodes.peek_any().not_eof("word")?;

        match peeked.node {
            TokenNode::Token(Token {
                unspanned: UnspannedToken::Bare,
                span,
            }) => {
                peeked.commit();
                Ok(BareSyntax {
                    word: context.source.to_string(),
                    span: *span,
                })
            }

            other => Err(ParseError::mismatch(
                "word",
                other.type_name().spanned(other.span()),
            )),
        }
    }
}

impl TestSyntax for BareShape {
    fn test<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        _context: &ExpandContext,
    ) -> Option<Peeked<'a, 'b>> {
        let peeked = token_nodes.peek_any();

        match peeked.node {
            Some(token) if token.is_bare() => Some(peeked),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum CommandSignature {
    Internal(Spanned<Signature>),
    LiteralExternal { outer: Span, inner: Span },
    External(Span),
    Expression(hir::Expression),
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
    pub fn to_expression(&self) -> hir::Expression {
        match self {
            CommandSignature::Internal(command) => {
                let span = command.span;
                hir::RawExpression::Command(span).into_expr(span)
            }
            CommandSignature::LiteralExternal { outer, inner } => {
                hir::RawExpression::ExternalCommand(hir::ExternalCommand::new(*inner))
                    .into_expr(*outer)
            }
            CommandSignature::External(span) => {
                hir::RawExpression::ExternalCommand(hir::ExternalCommand::new(*span))
                    .into_expr(*span)
            }
            CommandSignature::Expression(expr) => expr.clone(),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct PipelineShape;

// The failure mode is if the head of the token stream is not a pipeline
impl FallibleColorSyntax for PipelineShape {
    type Info = ();
    type Input = ();

    fn name(&self) -> &'static str {
        "PipelineShape"
    }

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<(), ShellError> {
        // Make sure we're looking at a pipeline
        let pipeline = token_nodes.peek_any_token("pipeline", |node| node.as_pipeline())?;

        let parts = &pipeline.parts[..];

        // Enumerate the pipeline parts
        for part in parts {
            // If the pipeline part has a prefix `|`, emit a pipe to color
            if let Some(pipe) = part.pipe {
                token_nodes.color_shape(FlatShape::Pipe.spanned(pipe))
            }

            let tokens: Spanned<&[TokenNode]> = (part.tokens()).spanned(part.span());

            token_nodes.child(tokens, context.source.clone(), move |token_nodes| {
                color_syntax(&MaybeSpaceShape, token_nodes, context);
                color_syntax(&CommandShape, token_nodes, context);
            });
        }

        Ok(())
    }
}

impl ExpandSyntax for PipelineShape {
    type Output = ClassifiedPipeline;

    fn name(&self) -> &'static str {
        "pipeline"
    }

    fn expand_syntax<'content, 'me>(
        &self,
        iterator: &'me mut TokensIterator<'content>,
        context: &ExpandContext,
    ) -> Result<Self::Output, ParseError> {
        let start = iterator.span_at_cursor();

        let peeked = iterator.peek_any().not_eof("pipeline")?;
        let pipeline = peeked.commit().as_pipeline()?;

        let parts = &pipeline.parts[..];

        let mut out = vec![];

        for part in parts {
            let tokens: Spanned<&[TokenNode]> = part.tokens().spanned(part.span());

            let classified =
                iterator.child(tokens, context.source.clone(), move |token_nodes| {
                    expand_syntax(&ClassifiedCommandShape, token_nodes, context)
                })?;

            out.push(classified);
        }

        let end = iterator.span_at_cursor();

        Ok(ClassifiedPipeline::commands(out, start.until(end)))
    }
}

pub enum CommandHeadKind {
    External,
    Internal(Signature),
}

#[derive(Debug, Copy, Clone)]
pub struct CommandHeadShape;

impl FallibleColorSyntax for CommandHeadShape {
    type Info = CommandHeadKind;
    type Input = ();

    fn name(&self) -> &'static str {
        "CommandHeadShape"
    }

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<CommandHeadKind, ShellError> {
        // If we don't ultimately find a token, roll back
        token_nodes.atomic(|token_nodes| {
            // First, take a look at the next token
            let atom = expand_atom(
                token_nodes,
                "command head",
                context,
                ExpansionRule::permissive(),
            )?;

            match atom.unspanned {
                // If the head is an explicit external command (^cmd), color it as an external command
                UnspannedAtomicToken::ExternalCommand { .. } => {
                    token_nodes.color_shape(FlatShape::ExternalCommand.spanned(atom.span));
                    Ok(CommandHeadKind::External)
                }

                // If the head is a word, it depends on whether it matches a registered internal command
                UnspannedAtomicToken::Word { text } => {
                    let name = text.slice(context.source);

                    if context.registry.has(name)? {
                        // If the registry has the command, color it as an internal command
                        token_nodes.color_shape(FlatShape::InternalCommand.spanned(text));
                        let signature = context
                            .registry
                            .get(name)
                            .map_err(|_| {
                                ShellError::labeled_error(
                                    "Internal error: could not load signature from registry",
                                    "could not load from registry",
                                    text,
                                )
                            })?
                            .ok_or_else(|| {
                                ShellError::labeled_error(
                                    "Internal error: could not load signature from registry",
                                    "could not load from registry",
                                    text,
                                )
                            })?;
                        Ok(CommandHeadKind::Internal(signature))
                    } else {
                        // Otherwise, color it as an external command
                        token_nodes.color_shape(FlatShape::ExternalCommand.spanned(text));
                        Ok(CommandHeadKind::External)
                    }
                }

                // Otherwise, we're not actually looking at a command
                _ => Err(ShellError::syntax_error(
                    "No command at the head".spanned(atom.span),
                )),
            }
        })
    }
}

impl ExpandSyntax for CommandHeadShape {
    type Output = CommandSignature;

    fn name(&self) -> &'static str {
        "command head"
    }

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
        context: &ExpandContext,
    ) -> Result<CommandSignature, ParseError> {
        let node =
            parse_single_node_skipping_ws(token_nodes, "command head1", |token, token_span, _| {
                Ok(match token {
                    UnspannedToken::ExternalCommand(span) => CommandSignature::LiteralExternal {
                        outer: token_span,
                        inner: span,
                    },
                    UnspannedToken::Bare => {
                        let name = token_span.slice(context.source);
                        if context.registry.has(name)? {
                            let signature = context
                                .registry
                                .get(name)
                                .map_err(|_| ParseError::internal_error(name.spanned(token_span)))?
                                .ok_or_else(|| {
                                    ParseError::internal_error(name.spanned(token_span))
                                })?;
                            CommandSignature::Internal(signature.spanned(token_span))
                        } else {
                            CommandSignature::External(token_span)
                        }
                    }
                    _ => {
                        return Err(ShellError::type_error(
                            "command head2",
                            token.type_name().spanned(token_span),
                        ))
                    }
                })
            });

        match node {
            Ok(expr) => Ok(expr),
            Err(_) => match expand_expr(&AnyExpressionShape, token_nodes, context) {
                Ok(expr) => Ok(CommandSignature::Expression(expr)),
                Err(_) => Err(token_nodes.peek_non_ws().type_error("command head3")),
            },
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ClassifiedCommandShape;

impl ExpandSyntax for ClassifiedCommandShape {
    type Output = ClassifiedCommand;

    fn name(&self) -> &'static str {
        "classified command"
    }

    fn expand_syntax<'a, 'b>(
        &self,
        iterator: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Self::Output, ParseError> {
        let start = iterator.span_at_cursor();
        let head = expand_syntax(&CommandHeadShape, iterator, context)?;

        match &head {
            CommandSignature::Expression(expr) => Err(ParseError::mismatch(
                "command",
                expr.type_name().spanned(expr.span),
            )),

            // If the command starts with `^`, treat it as an external command no matter what
            CommandSignature::External(name) => {
                let name_str = name.slice(&context.source);

                external_command(iterator, context, name_str.tagged(name))
            }

            CommandSignature::LiteralExternal { outer, inner } => {
                let name_str = inner.slice(&context.source);

                external_command(iterator, context, name_str.tagged(outer))
            }

            CommandSignature::Internal(signature) => {
                let tail = parse_command_tail(&signature.item, &context, iterator, signature.span)?;

                let (positional, named) = match tail {
                    None => (None, None),
                    Some((positional, named)) => (positional, named),
                };

                let end = iterator.span_at_cursor();

                let call = hir::Call {
                    head: Box::new(head.to_expression()),
                    positional,
                    named,
                    span: start.until(end),
                };

                Ok(ClassifiedCommand::Internal(InternalCommand::new(
                    signature.item.name.clone(),
                    Tag {
                        span: signature.span,
                        anchor: None,
                    },
                    call,
                )))
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct InternalCommandHeadShape;

impl FallibleColorSyntax for InternalCommandHeadShape {
    type Info = ();
    type Input = ();

    fn name(&self) -> &'static str {
        "InternalCommandHeadShape"
    }

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        _context: &ExpandContext,
    ) -> Result<(), ShellError> {
        let peeked_head = token_nodes.peek_non_ws().not_eof("command head4");

        let peeked_head = match peeked_head {
            Err(_) => return Ok(()),
            Ok(peeked_head) => peeked_head,
        };

        let node = peeked_head.commit();

        match node {
            TokenNode::Token(Token {
                unspanned: UnspannedToken::Bare,
                span,
            }) => token_nodes.color_shape(FlatShape::Word.spanned(*span)),

            TokenNode::Token(Token {
                unspanned: UnspannedToken::String(_inner_tag),
                span,
            }) => token_nodes.color_shape(FlatShape::String.spanned(*span)),

            _node => token_nodes.color_shape(FlatShape::Error.spanned(node.span())),
        };

        Ok(())
    }
}

impl ExpandExpression for InternalCommandHeadShape {
    fn name(&self) -> &'static str {
        "internal command head"
    }

    fn expand_expr(
        &self,
        token_nodes: &mut TokensIterator<'_>,
        _context: &ExpandContext,
    ) -> Result<hir::Expression, ParseError> {
        let peeked_head = token_nodes.peek_non_ws().not_eof("command head")?;

        let expr = match peeked_head.node {
            TokenNode::Token(Token {
                unspanned: UnspannedToken::Bare,
                span,
            }) => hir::RawExpression::Literal(hir::RawLiteral::Bare.into_literal(span))
                .into_expr(span),

            TokenNode::Token(Token {
                unspanned: UnspannedToken::String(inner_span),
                span,
            }) => {
                hir::RawExpression::Literal(hir::RawLiteral::String(*inner_span).into_literal(span))
                    .into_expr(span)
            }

            node => {
                return Err(ParseError::mismatch(
                    "command head",
                    node.type_name().spanned(node.span()),
                ))
            }
        };

        peeked_head.commit();

        Ok(expr)
    }
}

pub(crate) struct SingleError<'token> {
    expected: &'static str,
    node: &'token Token,
}

impl<'token> SingleError<'token> {
    pub(crate) fn error(&self) -> ParseError {
        ParseError::mismatch(self.expected, self.node.type_name().spanned(self.node.span))
    }
}

fn parse_single_node<T>(
    token_nodes: &mut TokensIterator<'_>,
    expected: &'static str,
    callback: impl FnOnce(UnspannedToken, Span, SingleError) -> Result<T, ParseError>,
) -> Result<T, ParseError> {
    token_nodes.peek_any_token(expected, |node| match node {
        TokenNode::Token(token) => callback(
            token.unspanned,
            token.span,
            SingleError {
                expected,
                node: token,
            },
        ),

        other => Err(ParseError::mismatch(
            expected,
            other.type_name().spanned(other.span()),
        )),
    })
}

fn parse_single_node_skipping_ws<T>(
    token_nodes: &mut TokensIterator<'_>,
    expected: &'static str,
    callback: impl FnOnce(UnspannedToken, Span, SingleError) -> Result<T, ShellError>,
) -> Result<T, ShellError> {
    let peeked = token_nodes.peek_non_ws().not_eof(expected)?;

    let expr = match peeked.node {
        TokenNode::Token(token) => callback(
            token.unspanned,
            token.span,
            SingleError {
                expected,
                node: token,
            },
        )?,

        other => {
            return Err(ShellError::type_error(
                expected,
                other.type_name().spanned(other.span()),
            ))
        }
    };

    peeked.commit();

    Ok(expr)
}

#[derive(Debug, Copy, Clone)]
pub struct WhitespaceShape;

impl FallibleColorSyntax for WhitespaceShape {
    type Info = ();
    type Input = ();

    fn name(&self) -> &'static str {
        "WhitespaceShape"
    }

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        _context: &ExpandContext,
    ) -> Result<(), ShellError> {
        let peeked = token_nodes.peek_any().not_eof("whitespace");

        let peeked = match peeked {
            Err(_) => return Ok(()),
            Ok(peeked) => peeked,
        };

        let node = peeked.commit();

        match node {
            TokenNode::Whitespace(span) => {
                token_nodes.color_shape(FlatShape::Whitespace.spanned(*span))
            }

            _other => return Ok(()),
        };

        Ok(())
    }
}

impl ExpandSyntax for WhitespaceShape {
    type Output = Span;

    fn name(&self) -> &'static str {
        "whitespace"
    }

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        _context: &ExpandContext,
    ) -> Result<Self::Output, ParseError> {
        let peeked = token_nodes.peek_any().not_eof("whitespace")?;

        let span = match peeked.node {
            TokenNode::Whitespace(tag) => *tag,

            other => {
                return Err(ParseError::mismatch(
                    "whitespace",
                    other.type_name().spanned(other.span()),
                ))
            }
        };

        peeked.commit();

        Ok(span)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct SpacedExpression<T: ExpandExpression> {
    inner: T,
}

impl<T: ExpandExpression> ExpandExpression for SpacedExpression<T> {
    fn name(&self) -> &'static str {
        "spaced expression"
    }

    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<hir::Expression, ParseError> {
        // TODO: Make the name part of the trait
        let peeked = token_nodes.peek_any().not_eof("whitespace")?;

        match peeked.node {
            TokenNode::Whitespace(_) => {
                peeked.commit();
                expand_expr(&self.inner, token_nodes, context)
            }

            other => Err(ParseError::mismatch(
                "whitespace",
                other.type_name().spanned(other.span()),
            )),
        }
    }
}

pub fn maybe_spaced<T: ExpandExpression>(inner: T) -> MaybeSpacedExpression<T> {
    MaybeSpacedExpression { inner }
}

#[derive(Debug, Copy, Clone)]
pub struct MaybeSpacedExpression<T: ExpandExpression> {
    inner: T,
}

#[derive(Debug, Copy, Clone)]
pub struct MaybeSpaceShape;

impl ExpandSyntax for MaybeSpaceShape {
    type Output = Option<Span>;

    fn name(&self) -> &'static str {
        "maybe space"
    }

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        _context: &ExpandContext,
    ) -> Result<Self::Output, ParseError> {
        let peeked = token_nodes.peek_any().not_eof("whitespace");

        let span = match peeked {
            Err(_) => None,
            Ok(peeked) => {
                if let TokenNode::Whitespace(..) = peeked.node {
                    let node = peeked.commit();
                    Some(node.span())
                } else {
                    None
                }
            }
        };

        Ok(span)
    }
}

impl ColorSyntax for MaybeSpaceShape {
    type Info = ();
    type Input = ();

    fn name(&self) -> &'static str {
        "MaybeSpaceShape"
    }

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        _context: &ExpandContext,
    ) -> Self::Info {
        let peeked = token_nodes.peek_any().not_eof("whitespace");

        let peeked = match peeked {
            Err(_) => return,
            Ok(peeked) => peeked,
        };

        if let TokenNode::Whitespace(span) = peeked.node {
            peeked.commit();
            token_nodes.color_shape(FlatShape::Whitespace.spanned(*span));
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct SpaceShape;

impl FallibleColorSyntax for SpaceShape {
    type Info = ();
    type Input = ();

    fn name(&self) -> &'static str {
        "SpaceShape"
    }

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        _context: &ExpandContext,
    ) -> Result<(), ShellError> {
        let peeked = token_nodes.peek_any().not_eof("whitespace")?;

        match peeked.node {
            TokenNode::Whitespace(span) => {
                peeked.commit();
                token_nodes.color_shape(FlatShape::Whitespace.spanned(*span));
                Ok(())
            }

            other => Err(ShellError::type_error(
                "whitespace",
                other.type_name().spanned(other.span()),
            )),
        }
    }
}

impl<T: ExpandExpression> ExpandExpression for MaybeSpacedExpression<T> {
    fn name(&self) -> &'static str {
        "maybe space"
    }

    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<hir::Expression, ParseError> {
        // TODO: Make the name part of the trait
        let peeked = token_nodes.peek_any().not_eof("whitespace")?;

        match peeked.node {
            TokenNode::Whitespace(_) => {
                peeked.commit();
                expand_expr(&self.inner, token_nodes, context)
            }

            _ => {
                peeked.rollback();
                expand_expr(&self.inner, token_nodes, context)
            }
        }
    }
}

pub fn spaced<T: ExpandExpression>(inner: T) -> SpacedExpression<T> {
    SpacedExpression { inner }
}

fn expand_variable(span: Span, token_span: Span, source: &Text) -> hir::Expression {
    if span.slice(source) == "it" {
        hir::Expression::it_variable(span, token_span)
    } else {
        hir::Expression::variable(span, token_span)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct CommandShape;

impl ColorSyntax for CommandShape {
    type Info = ();
    type Input = ();

    fn name(&self) -> &'static str {
        "CommandShape"
    }

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) {
        let kind = color_fallible_syntax(&CommandHeadShape, token_nodes, context);

        match kind {
            Err(_) => {
                // We didn't find a command, so we'll have to fall back to parsing this pipeline part
                // as a blob of undifferentiated expressions
                color_syntax(&ExpressionListShape, token_nodes, context);
            }

            Ok(CommandHeadKind::External) => {
                color_syntax(&ExternalTokensShape, token_nodes, context);
            }
            Ok(CommandHeadKind::Internal(signature)) => {
                color_syntax_with(&CommandTailShape, &signature, token_nodes, context);
            }
        };
    }
}
