use crate::parser::hir::syntax_shape::{
    expand_atom, expand_variable, parse_single_node, AtomicToken, ExpandContext, ExpandExpression,
    ExpansionRule, FallibleColorSyntax, FlatShape, TestSyntax,
};
use crate::parser::hir::tokens_iterator::Peeked;
use crate::parser::{hir, hir::TokensIterator, RawToken, TokenNode};
use crate::prelude::*;

#[derive(Debug, Copy, Clone)]
pub struct StringShape;

impl FallibleColorSyntax for StringShape {
    type Info = ();
    type Input = FlatShape;

    fn color_syntax<'a, 'b>(
        &self,
        input: &FlatShape,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
        shapes: &mut Vec<Tagged<FlatShape>>,
    ) -> Result<(), ShellError> {
        let atom = expand_atom(token_nodes, "string", context, ExpansionRule::permissive());

        let atom = match atom {
            Err(_) => return Ok(()),
            Ok(atom) => atom,
        };

        match atom {
            Tagged {
                item: AtomicToken::String { .. },
                tag,
            } => shapes.push((*input).tagged(tag)),
            other => other.color_tokens(shapes),
        }

        Ok(())
    }
}

impl ExpandExpression for StringShape {
    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
        context: &ExpandContext,
    ) -> Result<hir::Expression, ShellError> {
        parse_single_node(token_nodes, "String", |token, token_tag, _| {
            Ok(match token {
                RawToken::GlobPattern => {
                    return Err(ShellError::type_error(
                        "String",
                        "glob pattern".tagged(token_tag),
                    ))
                }
                RawToken::Operator(..) => {
                    return Err(ShellError::type_error(
                        "String",
                        "operator".tagged(token_tag),
                    ))
                }
                RawToken::Variable(tag) => expand_variable(tag, token_tag, &context.source),
                RawToken::ExternalCommand(tag) => hir::Expression::external_command(tag, token_tag),
                RawToken::ExternalWord => return Err(ShellError::invalid_external_word(token_tag)),
                RawToken::Number(_) => hir::Expression::bare(token_tag),
                RawToken::Bare => hir::Expression::bare(token_tag),
                RawToken::String(tag) => hir::Expression::string(tag, token_tag),
            })
        })
    }
}

impl TestSyntax for StringShape {
    fn test<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        _context: &ExpandContext,
    ) -> Option<Peeked<'a, 'b>> {
        let peeked = token_nodes.peek_any();

        match peeked.node {
            Some(TokenNode::Token(token)) => match token.item {
                RawToken::String(_) => Some(peeked),
                _ => None,
            },

            _ => None,
        }
    }
}
