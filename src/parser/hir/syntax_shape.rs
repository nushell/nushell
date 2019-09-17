mod block;
mod expression;

use crate::cli::external_command;
use crate::commands::{classified::InternalCommand, ClassifiedCommand, Command};
use crate::parser::hir::syntax_shape::block::AnyBlockShape;
use crate::parser::hir::tokens_iterator::Peeked;
use crate::parser::parse_command::parse_command_tail;
use crate::parser::{
    hir,
    hir::{debug_tokens, TokensIterator},
    Operator, RawToken, TokenNode,
};
use crate::prelude::*;
use derive_new::new;
use getset::Getters;
use log::trace;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub(crate) use self::expression::file_path::FilePathShape;
pub(crate) use self::expression::list::ExpressionListShape;
pub(crate) use self::expression::number::{IntShape, NumberShape};
pub(crate) use self::expression::pattern::PatternShape;
pub(crate) use self::expression::string::StringShape;
pub(crate) use self::expression::unit::UnitShape;
pub(crate) use self::expression::variable_path::{
    ColumnPathShape, DotShape, ExpressionContinuation, ExpressionContinuationShape, MemberShape,
    PathTailShape, VariablePathShape,
};
pub(crate) use self::expression::{continue_expression, AnyExpressionShape};

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum SyntaxShape {
    Any,
    List,
    String,
    Member,
    ColumnPath,
    Number,
    Int,
    Path,
    Pattern,
    Binary,
    Block,
    Boolean,
}

impl ExpandExpression for SyntaxShape {
    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<hir::Expression, ShellError> {
        match self {
            SyntaxShape::Any => expand_expr(&AnyExpressionShape, token_nodes, context),
            SyntaxShape::List => Err(ShellError::unimplemented("SyntaxShape:List")),
            SyntaxShape::Int => expand_expr(&IntShape, token_nodes, context),
            SyntaxShape::String => expand_expr(&StringShape, token_nodes, context),
            SyntaxShape::Member => {
                let syntax = expand_syntax(&MemberShape, token_nodes, context)?;
                Ok(syntax.to_expr())
            }
            SyntaxShape::ColumnPath => {
                let Tagged { item: members, tag } =
                    expand_syntax(&ColumnPathShape, token_nodes, context)?;

                Ok(hir::Expression::list(
                    members.into_iter().map(|s| s.to_expr()).collect(),
                    tag,
                ))
            }
            SyntaxShape::Number => expand_expr(&NumberShape, token_nodes, context),
            SyntaxShape::Path => expand_expr(&FilePathShape, token_nodes, context),
            SyntaxShape::Pattern => expand_expr(&PatternShape, token_nodes, context),
            SyntaxShape::Binary => Err(ShellError::unimplemented("SyntaxShape:Binary")),
            SyntaxShape::Block => expand_expr(&AnyBlockShape, token_nodes, context),
            SyntaxShape::Boolean => Err(ShellError::unimplemented("SyntaxShape:Boolean")),
        }
    }
}

impl std::fmt::Display for SyntaxShape {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SyntaxShape::Any => write!(f, "Any"),
            SyntaxShape::List => write!(f, "List"),
            SyntaxShape::String => write!(f, "String"),
            SyntaxShape::Int => write!(f, "Integer"),
            SyntaxShape::Member => write!(f, "Member"),
            SyntaxShape::ColumnPath => write!(f, "ColumnPath"),
            SyntaxShape::Number => write!(f, "Number"),
            SyntaxShape::Path => write!(f, "Path"),
            SyntaxShape::Pattern => write!(f, "Pattern"),
            SyntaxShape::Binary => write!(f, "Binary"),
            SyntaxShape::Block => write!(f, "Block"),
            SyntaxShape::Boolean => write!(f, "Boolean"),
        }
    }
}

#[derive(Getters, new)]
pub struct ExpandContext<'context> {
    #[get = "pub(crate)"]
    registry: &'context CommandRegistry,
    #[get = "pub(crate)"]
    tag: Tag,
    #[get = "pub(crate)"]
    source: &'context Text,
    homedir: Option<PathBuf>,
}

impl<'context> ExpandContext<'context> {
    pub(crate) fn homedir(&self) -> Option<&Path> {
        self.homedir.as_ref().map(|h| h.as_path())
    }

    #[cfg(test)]
    pub fn with_empty(source: &Text, callback: impl FnOnce(ExpandContext)) {
        let mut registry = CommandRegistry::new();
        registry.insert(
            "ls",
            crate::commands::whole_stream_command(crate::commands::LS),
        );

        callback(ExpandContext {
            registry: &registry,
            tag: Tag::unknown(),
            source,
            homedir: None,
        })
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
    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<hir::Expression, ShellError>;
}

pub(crate) trait ExpandSyntax: std::fmt::Debug + Copy {
    type Output: std::fmt::Debug;

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Self::Output, ShellError>;
}

pub(crate) fn expand_syntax<'a, 'b, T: ExpandSyntax>(
    shape: &T,
    token_nodes: &'b mut TokensIterator<'a>,
    context: &ExpandContext,
) -> Result<T::Output, ShellError> {
    trace!(target: "nu::expand_syntax", "before {} :: {:?}", std::any::type_name::<T>(), debug_tokens(token_nodes, context.source));

    let result = shape.expand_syntax(token_nodes, context);

    match result {
        Err(err) => {
            trace!(target: "nu::expand_syntax", "error :: {} :: {:?}", err, debug_tokens(token_nodes, context.source));
            Err(err)
        }

        Ok(result) => {
            trace!(target: "nu::expand_syntax", "ok :: {:?} :: {:?}", result, debug_tokens(token_nodes, context.source));
            Ok(result)
        }
    }
}

pub(crate) fn expand_expr<'a, 'b, T: ExpandExpression>(
    shape: &T,
    token_nodes: &'b mut TokensIterator<'a>,
    context: &ExpandContext,
) -> Result<hir::Expression, ShellError> {
    trace!(target: "nu::expand_syntax", "before {} :: {:?}", std::any::type_name::<T>(), debug_tokens(token_nodes, context.source));

    let result = shape.expand_syntax(token_nodes, context);

    match result {
        Err(err) => {
            trace!(target: "nu::expand_syntax", "error :: {} :: {:?}", err, debug_tokens(token_nodes, context.source));
            Err(err)
        }

        Ok(result) => {
            trace!(target: "nu::expand_syntax", "ok :: {:?} :: {:?}", result, debug_tokens(token_nodes, context.source));
            Ok(result)
        }
    }
}

impl<T: ExpandExpression> ExpandSyntax for T {
    type Output = hir::Expression;

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Self::Output, ShellError> {
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
    Seen(Tag, Tag),
    Error(ShellError),
}

impl BarePathState {
    pub fn seen(self, tag: Tag) -> BarePathState {
        match self {
            BarePathState::Initial => BarePathState::Seen(tag, tag),
            BarePathState::Seen(start, _) => BarePathState::Seen(start, tag),
            BarePathState::Error(err) => BarePathState::Error(err),
        }
    }

    pub fn end(self, peeked: Peeked, reason: impl Into<String>) -> BarePathState {
        match self {
            BarePathState::Initial => BarePathState::Error(peeked.type_error(reason)),
            BarePathState::Seen(start, end) => BarePathState::Seen(start, end),
            BarePathState::Error(err) => BarePathState::Error(err),
        }
    }

    pub fn into_bare(self) -> Result<Tag, ShellError> {
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
) -> Result<Tag, ShellError> {
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
                    state = state.seen(node.tag());
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
    type Output = Tag;

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Tag, ShellError> {
        expand_bare(token_nodes, context, |token| match token {
            TokenNode::Token(Tagged {
                item: RawToken::Bare,
                ..
            })
            | TokenNode::Token(Tagged {
                item: RawToken::Operator(Operator::Dot),
                ..
            }) => true,

            _ => false,
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub struct BareShape;

impl ExpandSyntax for BareShape {
    type Output = Tagged<String>;

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Self::Output, ShellError> {
        let peeked = token_nodes.peek_any().not_eof("word")?;

        match peeked.node {
            TokenNode::Token(Tagged {
                item: RawToken::Bare,
                tag,
            }) => {
                peeked.commit();
                Ok(tag.tagged_string(context.source))
            }

            other => Err(ShellError::type_error("word", other.tagged_type_name())),
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
            Some(TokenNode::Token(token)) => match token.item {
                RawToken::Bare => Some(peeked),
                _ => None,
            },

            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum CommandSignature {
    Internal(Tagged<Arc<Command>>),
    LiteralExternal { outer: Tag, inner: Tag },
    External(Tag),
    Expression(hir::Expression),
}

impl CommandSignature {
    pub fn to_expression(&self) -> hir::Expression {
        match self {
            CommandSignature::Internal(command) => {
                let tag = command.tag;
                hir::RawExpression::Command(tag).tagged(tag)
            }
            CommandSignature::LiteralExternal { outer, inner } => {
                hir::RawExpression::ExternalCommand(hir::ExternalCommand::new(*inner)).tagged(outer)
            }
            CommandSignature::External(tag) => {
                hir::RawExpression::ExternalCommand(hir::ExternalCommand::new(*tag)).tagged(tag)
            }
            CommandSignature::Expression(expr) => expr.clone(),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct CommandHeadShape;

impl ExpandSyntax for CommandHeadShape {
    type Output = CommandSignature;

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
        context: &ExpandContext,
    ) -> Result<CommandSignature, ShellError> {
        let node =
            parse_single_node_skipping_ws(token_nodes, "command head1", |token, token_tag| {
                Ok(match token {
                    RawToken::ExternalCommand(tag) => CommandSignature::LiteralExternal {
                        outer: token_tag,
                        inner: tag,
                    },
                    RawToken::Bare => {
                        let name = token_tag.slice(context.source);
                        if context.registry.has(name) {
                            let command = context.registry.expect_command(name);
                            CommandSignature::Internal(command.tagged(token_tag))
                        } else {
                            CommandSignature::External(token_tag)
                        }
                    }
                    _ => {
                        return Err(ShellError::type_error(
                            "command head2",
                            token.type_name().tagged(token_tag),
                        ))
                    }
                })
            });

        match node {
            Ok(expr) => return Ok(expr),
            Err(_) => match expand_expr(&AnyExpressionShape, token_nodes, context) {
                Ok(expr) => return Ok(CommandSignature::Expression(expr)),
                Err(_) => Err(token_nodes.peek_non_ws().type_error("command head3")),
            },
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ClassifiedCommandShape;

impl ExpandSyntax for ClassifiedCommandShape {
    type Output = ClassifiedCommand;

    fn expand_syntax<'a, 'b>(
        &self,
        iterator: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Self::Output, ShellError> {
        let head = expand_syntax(&CommandHeadShape, iterator, context)?;

        match &head {
            CommandSignature::Expression(expr) => Err(ShellError::syntax_error(
                "Unexpected expression in command position".tagged(expr.tag),
            )),

            // If the command starts with `^`, treat it as an external command no matter what
            CommandSignature::External(name) => {
                let name_str = name.slice(&context.source);

                external_command(iterator, &context.source, name_str.tagged(name))
            }

            CommandSignature::LiteralExternal { outer, inner } => {
                let name_str = inner.slice(&context.source);

                external_command(iterator, &context.source, name_str.tagged(outer))
            }

            CommandSignature::Internal(command) => {
                let tail =
                    parse_command_tail(&command.signature(), &context, iterator, command.tag)?;

                let (positional, named) = match tail {
                    None => (None, None),
                    Some((positional, named)) => (positional, named),
                };

                let call = hir::Call {
                    head: Box::new(head.to_expression()),
                    positional,
                    named,
                };

                Ok(ClassifiedCommand::Internal(InternalCommand::new(
                    command.item.name().to_string(),
                    command.tag,
                    call,
                )))
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct InternalCommandHeadShape;

impl ExpandExpression for InternalCommandHeadShape {
    fn expand_expr(
        &self,
        token_nodes: &mut TokensIterator<'_>,
        _context: &ExpandContext,
    ) -> Result<hir::Expression, ShellError> {
        let peeked_head = token_nodes.peek_non_ws().not_eof("command head4")?;

        let expr = match peeked_head.node {
            TokenNode::Token(
                spanned @ Tagged {
                    item: RawToken::Bare,
                    ..
                },
            ) => spanned.map(|_| hir::RawExpression::Literal(hir::Literal::Bare)),

            TokenNode::Token(Tagged {
                item: RawToken::String(inner_tag),
                tag,
            }) => hir::RawExpression::Literal(hir::Literal::String(*inner_tag)).tagged(*tag),

            node => {
                return Err(ShellError::type_error(
                    "command head5",
                    node.tagged_type_name(),
                ))
            }
        };

        peeked_head.commit();

        Ok(expr)
    }
}

fn parse_single_node<'a, 'b, T>(
    token_nodes: &'b mut TokensIterator<'a>,
    expected: &'static str,
    callback: impl FnOnce(RawToken, Tag) -> Result<T, ShellError>,
) -> Result<T, ShellError> {
    let peeked = token_nodes.peek_any().not_eof(expected)?;

    let expr = match peeked.node {
        TokenNode::Token(token) => callback(token.item, token.tag())?,

        other => return Err(ShellError::type_error(expected, other.tagged_type_name())),
    };

    peeked.commit();

    Ok(expr)
}

fn parse_single_node_skipping_ws<'a, 'b, T>(
    token_nodes: &'b mut TokensIterator<'a>,
    expected: &'static str,
    callback: impl FnOnce(RawToken, Tag) -> Result<T, ShellError>,
) -> Result<T, ShellError> {
    let peeked = token_nodes.peek_non_ws().not_eof(expected)?;

    let expr = match peeked.node {
        TokenNode::Token(token) => callback(token.item, token.tag())?,

        other => return Err(ShellError::type_error(expected, other.tagged_type_name())),
    };

    peeked.commit();

    Ok(expr)
}

#[derive(Debug, Copy, Clone)]
pub struct WhitespaceShape;

impl ExpandSyntax for WhitespaceShape {
    type Output = Tag;

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        _context: &ExpandContext,
    ) -> Result<Self::Output, ShellError> {
        let peeked = token_nodes.peek_any().not_eof("whitespace")?;

        let tag = match peeked.node {
            TokenNode::Whitespace(tag) => *tag,

            other => {
                return Err(ShellError::type_error(
                    "whitespace",
                    other.tagged_type_name(),
                ))
            }
        };

        peeked.commit();

        Ok(tag)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct SpacedExpression<T: ExpandExpression> {
    inner: T,
}

impl<T: ExpandExpression> ExpandExpression for SpacedExpression<T> {
    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<hir::Expression, ShellError> {
        // TODO: Make the name part of the trait
        let peeked = token_nodes.peek_any().not_eof("whitespace")?;

        match peeked.node {
            TokenNode::Whitespace(_) => {
                peeked.commit();
                expand_expr(&self.inner, token_nodes, context)
            }

            other => Err(ShellError::type_error(
                "whitespace",
                other.tagged_type_name(),
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

impl<T: ExpandExpression> ExpandExpression for MaybeSpacedExpression<T> {
    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<hir::Expression, ShellError> {
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

fn expand_variable(tag: Tag, token_tag: Tag, source: &Text) -> hir::Expression {
    if tag.slice(source) == "it" {
        hir::Expression::it_variable(tag, token_tag)
    } else {
        hir::Expression::variable(tag, token_tag)
    }
}
