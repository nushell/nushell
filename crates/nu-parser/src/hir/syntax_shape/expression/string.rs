use crate::hir::syntax_shape::{
    expand_atom, expand_variable, parse_single_node, AtomicToken, ExpandContext, ExpandExpression,
    ExpansionRule, FallibleColorSyntax, FlatShape, TestSyntax, UnspannedAtomicToken,
};
use crate::hir::tokens_iterator::Peeked;
use crate::parse::tokens::UnspannedToken;
use crate::{hir, hir::TokensIterator};
use nu_errors::{ParseError, ShellError};
use nu_source::SpannedItem;

#[derive(Debug, Copy, Clone)]
pub struct StringShape;

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
            AtomicToken {
                unspanned: UnspannedAtomicToken::String { .. },
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
                UnspannedToken::GlobPattern
                | UnspannedToken::CompareOperator(..)
                | UnspannedToken::EvaluationOperator(..)
                | UnspannedToken::ExternalWord => return Err(err.error()),
                UnspannedToken::Variable(span) => {
                    expand_variable(span, token_span, &context.source)
                }
                UnspannedToken::ExternalCommand(span) => {
                    hir::Expression::external_command(span, token_span)
                }
                UnspannedToken::Number(_) => hir::Expression::bare(token_span),
                UnspannedToken::Bare => hir::Expression::bare(token_span),
                UnspannedToken::String(span) => hir::Expression::string(span, token_span),
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
