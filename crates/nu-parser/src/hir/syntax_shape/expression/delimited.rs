use crate::hir::syntax_shape::ExpandSyntax;
use crate::hir::SpannedExpression;
use crate::{hir, hir::TokensIterator};
use nu_errors::ParseError;

#[derive(Debug, Copy, Clone)]
pub struct DelimitedSquareShape;

impl ExpandSyntax for DelimitedSquareShape {
    type Output = Result<SpannedExpression, ParseError>;

    fn name(&self) -> &'static str {
        "delimited square"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
    ) -> Result<SpannedExpression, ParseError> {
        let exprs = token_nodes.square()?;

        Ok(hir::Expression::list(exprs.item).into_expr(exprs.span))
    }
}
