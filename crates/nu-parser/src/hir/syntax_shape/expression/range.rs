use crate::hir::syntax_shape::expression::UnspannedAtomicToken;
use crate::hir::syntax_shape::{
    color_fallible_syntax, expand_atom, expand_expr, AnyExpressionShape, ExpandContext,
    ExpandExpression, ExpansionRule, FallibleColorSyntax, FlatShape,
};
use crate::parse::operator::EvaluationOperator;
use crate::parse::token_tree::TokenNode;
use crate::parse::tokens::{Token, UnspannedToken};
use crate::{hir, hir::TokensIterator};
use nu_errors::{ParseError, ShellError};
use nu_protocol::SpannedTypeName;
use nu_source::SpannedItem;

#[derive(Debug, Copy, Clone)]
pub struct RangeShape;

impl ExpandExpression for RangeShape {
    fn name(&self) -> &'static str {
        "range"
    }

    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<hir::Expression, ParseError> {
        token_nodes.atomic_parse(|token_nodes| {
            let left = expand_expr(&AnyExpressionShape, token_nodes, context)?;

            let atom = expand_atom(
                token_nodes,
                "..",
                context,
                ExpansionRule::new().allow_eval_operator(),
            )?;

            let span = match atom.unspanned {
                UnspannedAtomicToken::DotDot { text } => text,
                _ => return Err(ParseError::mismatch("..", atom.spanned_type_name())),
            };

            let right = expand_expr(&AnyExpressionShape, token_nodes, context)?;

            Ok(hir::Expression::range(left, span, right))
        })
    }
}

impl FallibleColorSyntax for RangeShape {
    type Info = ();
    type Input = ();

    fn name(&self) -> &'static str {
        "RangeShape"
    }

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<(), ShellError> {
        token_nodes.atomic_parse(|token_nodes| {
            color_fallible_syntax(&AnyExpressionShape, token_nodes, context)?;
            color_fallible_syntax(&DotDotShape, token_nodes, context)?;
            color_fallible_syntax(&AnyExpressionShape, token_nodes, context)
        })?;

        Ok(())
    }
}

#[derive(Debug, Copy, Clone)]
struct DotDotShape;

impl FallibleColorSyntax for DotDotShape {
    type Info = ();
    type Input = ();

    fn name(&self) -> &'static str {
        ".."
    }

    fn color_syntax<'a, 'b>(
        &self,
        _input: &Self::Input,
        token_nodes: &'b mut TokensIterator<'a>,
        _context: &ExpandContext,
    ) -> Result<Self::Info, ShellError> {
        let peeked = token_nodes.peek_any().not_eof("..")?;
        match &peeked.node {
            TokenNode::Token(Token {
                unspanned: UnspannedToken::EvaluationOperator(EvaluationOperator::DotDot),
                span,
            }) => {
                peeked.commit();
                token_nodes.color_shape(FlatShape::DotDot.spanned(span));
                Ok(())
            }
            token => Err(ShellError::type_error("..", token.spanned_type_name())),
        }
    }
}
