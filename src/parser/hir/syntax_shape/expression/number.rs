use crate::parser::hir::syntax_shape::{
    expand_atom, parse_single_node, ExpandContext, ExpandExpression, ExpansionRule,
    FallibleColorSyntax, FlatShape,
};
use crate::parser::{
    hir,
    hir::{RawNumber, TokensIterator},
    RawToken,
};
use crate::prelude::*;

#[derive(Debug, Copy, Clone)]
pub struct NumberShape;

impl ExpandExpression for NumberShape {
    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
        context: &ExpandContext,
    ) -> Result<hir::Expression, ShellError> {
        parse_single_node(token_nodes, "Number", |token, token_tag, err| {
            Ok(match token {
                RawToken::GlobPattern | RawToken::Operator(..) => return Err(err.error()),
                RawToken::Variable(tag) if tag.slice(context.source) == "it" => {
                    hir::Expression::it_variable(tag, token_tag)
                }
                RawToken::ExternalCommand(tag) => hir::Expression::external_command(tag, token_tag),
                RawToken::ExternalWord => return Err(ShellError::invalid_external_word(token_tag)),
                RawToken::Variable(tag) => hir::Expression::variable(tag, token_tag),
                RawToken::Number(number) => {
                    hir::Expression::number(number.to_number(context.source), token_tag)
                }
                RawToken::Bare => hir::Expression::bare(token_tag),
                RawToken::String(tag) => hir::Expression::string(tag, token_tag),
            })
        })
    }
}

impl FallibleColorSyntax for NumberShape {
    type Info = ();
    type Input = ();

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
        shapes: &mut Vec<Tagged<FlatShape>>,
    ) -> Result<(), ShellError> {
        let atom = token_nodes.spanned(|token_nodes| {
            expand_atom(token_nodes, "number", context, ExpansionRule::permissive())
        });

        let atom = match atom {
            Tagged { item: Err(_), tag } => {
                shapes.push(FlatShape::Error.tagged(tag));
                return Ok(());
            }
            Tagged { item: Ok(atom), .. } => atom,
        };

        atom.color_tokens(shapes);

        Ok(())
    }
}

#[derive(Debug, Copy, Clone)]
pub struct IntShape;

impl ExpandExpression for IntShape {
    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
        context: &ExpandContext,
    ) -> Result<hir::Expression, ShellError> {
        parse_single_node(token_nodes, "Integer", |token, token_tag, err| {
            Ok(match token {
                RawToken::GlobPattern | RawToken::Operator(..) => return Err(err.error()),
                RawToken::ExternalWord => return Err(ShellError::invalid_external_word(token_tag)),
                RawToken::Variable(tag) if tag.slice(context.source) == "it" => {
                    hir::Expression::it_variable(tag, token_tag)
                }
                RawToken::ExternalCommand(tag) => hir::Expression::external_command(tag, token_tag),
                RawToken::Variable(tag) => hir::Expression::variable(tag, token_tag),
                RawToken::Number(number @ RawNumber::Int(_)) => {
                    hir::Expression::number(number.to_number(context.source), token_tag)
                }
                RawToken::Number(_) => return Err(err.error()),
                RawToken::Bare => hir::Expression::bare(token_tag),
                RawToken::String(tag) => hir::Expression::string(tag, token_tag),
            })
        })
    }
}

impl FallibleColorSyntax for IntShape {
    type Info = ();
    type Input = ();

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
        shapes: &mut Vec<Tagged<FlatShape>>,
    ) -> Result<(), ShellError> {
        let atom = token_nodes.spanned(|token_nodes| {
            expand_atom(token_nodes, "integer", context, ExpansionRule::permissive())
        });

        let atom = match atom {
            Tagged { item: Err(_), tag } => {
                shapes.push(FlatShape::Error.tagged(tag));
                return Ok(());
            }
            Tagged { item: Ok(atom), .. } => atom,
        };

        atom.color_tokens(shapes);

        Ok(())
    }
}
