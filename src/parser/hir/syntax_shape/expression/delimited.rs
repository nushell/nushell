use crate::parser::hir::syntax_shape::{expand_syntax, ExpandContext, ExpressionListShape};
use crate::parser::{hir, hir::TokensIterator};
use crate::parser::{DelimitedNode, Delimiter};
use crate::prelude::*;

pub fn expand_delimited_expr(
    delimited: &Tagged<DelimitedNode>,
    context: &ExpandContext,
) -> Result<hir::Expression, ShellError> {
    match &delimited.item {
        DelimitedNode {
            delimiter: Delimiter::Square,
            children,
        } => {
            let mut tokens = TokensIterator::new(&children, delimited.tag, false);

            let list = expand_syntax(&ExpressionListShape, &mut tokens, context);

            Ok(hir::Expression::list(list?, delimited.tag))
        }

        DelimitedNode {
            delimiter: Delimiter::Paren,
            ..
        } => Err(ShellError::type_error(
            "expression",
            "unimplemented call expression".tagged(delimited.tag),
        )),

        DelimitedNode {
            delimiter: Delimiter::Brace,
            ..
        } => Err(ShellError::type_error(
            "expression",
            "unimplemented block expression".tagged(delimited.tag),
        )),
    }
}
