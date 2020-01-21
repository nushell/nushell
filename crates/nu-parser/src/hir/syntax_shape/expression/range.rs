use crate::hir::syntax_shape::{AnyExpressionStartShape, ExpandSyntax, FlatShape};
use crate::hir::TokensIterator;
use crate::hir::{Expression, SpannedExpression};
use crate::parse::token_tree::DotDotType;
use nu_errors::ParseError;
use nu_source::{HasSpan, Span};

#[derive(Debug, Copy, Clone)]
pub struct RangeShape;

impl ExpandSyntax for RangeShape {
    type Output = Result<SpannedExpression, ParseError>;

    fn name(&self) -> &'static str {
        "range"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
    ) -> Result<SpannedExpression, ParseError> {
        token_nodes.atomic_parse(|token_nodes| {
            let left = token_nodes.expand_syntax(AnyExpressionStartShape)?;
            let dotdot = token_nodes.expand_syntax(DotDotShape)?;
            let right = token_nodes.expand_syntax(AnyExpressionStartShape)?;

            let span = left.span.until(right.span);

            Ok(Expression::range(left, dotdot, right).into_expr(span))
        })
    }
}

#[derive(Debug, Copy, Clone)]
struct DotDotShape;

impl ExpandSyntax for DotDotShape {
    type Output = Result<Span, ParseError>;

    fn name(&self) -> &'static str {
        "dotdot"
    }

    fn expand<'a, 'b>(&self, token_nodes: &'b mut TokensIterator<'a>) -> Result<Span, ParseError> {
        token_nodes.expand_token(DotDotType, |token| Ok((FlatShape::DotDot, token.span())))
    }
}
