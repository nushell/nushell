use crate::parser::hir::syntax_shape::{
    expand_atom, expand_variable, parse_single_node, AtomicToken, ExpandContext, ExpandExpression,
    ExpansionRule, FallibleColorSyntax, FlatShape, ParseError, TestSyntax,
};
use crate::parser::hir::tokens_iterator::Peeked;
use crate::parser::{hir, hir::TokensIterator, RawToken};
use crate::prelude::*;

#[derive(Debug, Copy, Clone)]
pub struct StringShape;

#[cfg(not(coloring_in_tokens))]
impl FallibleColorSyntax for StringShape {
    type Info = ();
    type Input = FlatShape;

    fn color_syntax<'a, 'b>(
        &self,
        input: &FlatShape,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
        shapes: &mut Vec<Spanned<FlatShape>>,
    ) -> Result<(), ShellError> {
        let atom = expand_atom(token_nodes, "string", context, ExpansionRule::permissive());

        let atom = match atom {
            Err(_) => return Ok(()),
            Ok(atom) => atom,
        };

        match atom {
            Spanned {
                item: AtomicToken::String { .. },
                span,
            } => shapes.push((*input).spanned(span)),
            other => other.color_tokens(shapes),
        }

        Ok(())
    }
}

#[cfg(coloring_in_tokens)]
impl FallibleColorSyntax for StringShape {
    type Info = ();
    type Input = FlatShape;

    fn name(&self) -> &'static str {
        "StringShape"
    }

    fn color_syntax<'a, 'b>(
        &self,
        input: &FlatShape,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<(), ShellError> {
        let atom = expand_atom(token_nodes, "string", context, ExpansionRule::permissive());

        let atom = match atom {
            Err(_) => return Ok(()),
            Ok(atom) => atom,
        };

        match atom {
            Spanned {
                item: AtomicToken::String { .. },
                span,
            } => token_nodes.color_shape((*input).spanned(span)),
            atom => token_nodes.mutate_shapes(|shapes| atom.color_tokens(shapes)),
        }

        Ok(())
    }
}

impl ExpandExpression for StringShape {
    fn name(&self) -> &'static str {
        "string"
    }

    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
        context: &ExpandContext,
    ) -> Result<hir::Expression, ParseError> {
        parse_single_node(token_nodes, "String", |token, token_span, err| {
            Ok(match token {
                RawToken::GlobPattern | RawToken::Operator(..) | RawToken::ExternalWord => {
                    return Err(err.error())
                }
                RawToken::Variable(span) => expand_variable(span, token_span, &context.source),
                RawToken::ExternalCommand(span) => {
                    hir::Expression::external_command(span, token_span)
                }
                RawToken::Number(_) => hir::Expression::bare(token_span),
                RawToken::Bare => hir::Expression::bare(token_span),
                RawToken::String(span) => hir::Expression::string(span, token_span),
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
            Some(token) if token.is_string() => Some(peeked),
            _ => None,
        }
    }
}
