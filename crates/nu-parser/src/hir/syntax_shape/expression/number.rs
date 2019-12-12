use crate::hir::syntax_shape::{
    expand_atom, parse_single_node, ExpandContext, ExpandExpression, ExpansionRule,
    FallibleColorSyntax, FlatShape, TestSyntax,
};
use crate::hir::tokens_iterator::Peeked;
use crate::parse::tokens::UnspannedToken;
use crate::{
    hir,
    hir::{RawNumber, TokensIterator},
};
use nu_errors::{ParseError, ShellError};
use nu_source::{Spanned, SpannedItem};

#[derive(Debug, Copy, Clone)]
pub struct NumberShape;

impl ExpandExpression for NumberShape {
    fn name(&self) -> &'static str {
        "number"
    }

    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
        context: &ExpandContext,
    ) -> Result<hir::Expression, ParseError> {
        parse_single_node(token_nodes, "Number", |token, token_span, err| {
            Ok(match token {
                UnspannedToken::GlobPattern
                | UnspannedToken::CompareOperator(..)
                | UnspannedToken::EvaluationOperator(..) => return Err(err.error()),
                UnspannedToken::Variable(tag) if tag.slice(context.source) == "it" => {
                    hir::Expression::it_variable(tag, token_span)
                }
                UnspannedToken::ExternalCommand(tag) => {
                    hir::Expression::external_command(tag, token_span)
                }
                UnspannedToken::ExternalWord => {
                    return Err(ParseError::mismatch(
                        "number",
                        "syntax error".spanned(token_span),
                    ))
                }
                UnspannedToken::Variable(tag) => hir::Expression::variable(tag, token_span),
                UnspannedToken::Number(number) => {
                    hir::Expression::number(number.to_number(context.source), token_span)
                }
                UnspannedToken::Bare => hir::Expression::bare(token_span),
                UnspannedToken::String(tag) => hir::Expression::string(tag, token_span),
            })
        })
    }
}

impl FallibleColorSyntax for NumberShape {
    type Info = ();
    type Input = ();

    fn name(&self) -> &'static str {
        "NumberShape"
    }

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<(), ShellError> {
        let atom = token_nodes.spanned(|token_nodes| {
            expand_atom(token_nodes, "number", context, ExpansionRule::permissive())
        });

        let atom = match atom {
            Spanned { item: Err(_), span } => {
                token_nodes.color_shape(FlatShape::Error.spanned(span));
                return Ok(());
            }
            Spanned { item: Ok(atom), .. } => atom,
        };

        token_nodes.mutate_shapes(|shapes| atom.color_tokens(shapes));

        Ok(())
    }
}

#[derive(Debug, Copy, Clone)]
pub struct IntShape;

impl ExpandExpression for IntShape {
    fn name(&self) -> &'static str {
        "integer"
    }

    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
        context: &ExpandContext,
    ) -> Result<hir::Expression, ParseError> {
        parse_single_node(token_nodes, "Integer", |token, token_span, err| {
            Ok(match token {
                UnspannedToken::GlobPattern
                | UnspannedToken::CompareOperator(..)
                | UnspannedToken::EvaluationOperator(..)
                | UnspannedToken::ExternalWord => return Err(err.error()),
                UnspannedToken::Variable(span) if span.slice(context.source) == "it" => {
                    hir::Expression::it_variable(span, token_span)
                }
                UnspannedToken::ExternalCommand(span) => {
                    hir::Expression::external_command(span, token_span)
                }
                UnspannedToken::Variable(span) => hir::Expression::variable(span, token_span),
                UnspannedToken::Number(number @ RawNumber::Int(_)) => {
                    hir::Expression::number(number.to_number(context.source), token_span)
                }
                UnspannedToken::Number(_) => return Err(err.error()),
                UnspannedToken::Bare => hir::Expression::bare(token_span),
                UnspannedToken::String(span) => hir::Expression::string(span, token_span),
            })
        })
    }
}

impl FallibleColorSyntax for IntShape {
    type Info = ();
    type Input = ();

    fn name(&self) -> &'static str {
        "IntShape"
    }

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<(), ShellError> {
        let atom = token_nodes.spanned(|token_nodes| {
            expand_atom(token_nodes, "integer", context, ExpansionRule::permissive())
        });

        let atom = match atom {
            Spanned { item: Err(_), span } => {
                token_nodes.color_shape(FlatShape::Error.spanned(span));
                return Ok(());
            }
            Spanned { item: Ok(atom), .. } => atom,
        };

        token_nodes.mutate_shapes(|shapes| atom.color_tokens(shapes));

        Ok(())
    }
}

impl TestSyntax for NumberShape {
    fn test<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        _context: &ExpandContext,
    ) -> Option<Peeked<'a, 'b>> {
        let peeked = token_nodes.peek_any();

        match peeked.node {
            Some(token) if token.is_number() => Some(peeked),
            _ => None,
        }
    }
}
