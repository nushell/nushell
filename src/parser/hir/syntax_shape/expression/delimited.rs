use crate::parser::hir::syntax_shape::{
    color_syntax, expand_syntax, ColorSyntax, ExpandContext, ExpressionListShape, TokenNode,
};
use crate::parser::{hir, hir::TokensIterator, Delimiter, FlatShape};
use crate::prelude::*;

pub fn expand_delimited_square(
    children: &Vec<TokenNode>,
    tag: Tag,
    context: &ExpandContext,
) -> Result<hir::Expression, ShellError> {
    let mut tokens = TokensIterator::new(&children, tag, false);

    let list = expand_syntax(&ExpressionListShape, &mut tokens, context);

    Ok(hir::Expression::list(list?, tag))
}

pub fn color_delimited_square(
    (open, close): (Tag, Tag),
    children: &Vec<TokenNode>,
    tag: Tag,
    context: &ExpandContext,
    shapes: &mut Vec<Tagged<FlatShape>>,
) {
    shapes.push(FlatShape::OpenDelimiter(Delimiter::Square).tagged(open));
    let mut tokens = TokensIterator::new(&children, tag, false);
    let _list = color_syntax(&ExpressionListShape, &mut tokens, context, shapes);
    shapes.push(FlatShape::CloseDelimiter(Delimiter::Square).tagged(close));
}

#[derive(Debug, Copy, Clone)]
pub struct DelimitedShape;

impl ColorSyntax for DelimitedShape {
    type Info = ();
    type Input = (Delimiter, Tag, Tag);
    fn color_syntax<'a, 'b>(
        &self,
        (delimiter, open, close): &(Delimiter, Tag, Tag),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
        shapes: &mut Vec<Tagged<FlatShape>>,
    ) -> Self::Info {
        shapes.push(FlatShape::OpenDelimiter(*delimiter).tagged(open));
        color_syntax(&ExpressionListShape, token_nodes, context, shapes);
        shapes.push(FlatShape::CloseDelimiter(*delimiter).tagged(close));
    }
}
