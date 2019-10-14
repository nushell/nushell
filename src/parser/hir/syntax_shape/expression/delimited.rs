use crate::parser::hir::syntax_shape::{
    color_syntax, expand_syntax, ColorSyntax, ExpandContext, ExpressionListShape, TokenNode,
};
use crate::parser::{hir, hir::TokensIterator, Delimiter, FlatShape};
use crate::prelude::*;

pub fn expand_delimited_square(
    children: &Vec<TokenNode>,
    span: Span,
    context: &ExpandContext,
) -> Result<hir::Expression, ShellError> {
    let mut tokens = TokensIterator::new(&children, span, false);

    let list = expand_syntax(&ExpressionListShape, &mut tokens, context);

    Ok(hir::Expression::list(list?, Tag { span, anchor: None }))
}

pub fn color_delimited_square(
    (open, close): (Span, Span),
    children: &Vec<TokenNode>,
    span: Span,
    context: &ExpandContext,
    shapes: &mut Vec<Spanned<FlatShape>>,
) {
    shapes.push(FlatShape::OpenDelimiter(Delimiter::Square).spanned(open));
    let mut tokens = TokensIterator::new(&children, span, false);
    let _list = color_syntax(&ExpressionListShape, &mut tokens, context, shapes);
    shapes.push(FlatShape::CloseDelimiter(Delimiter::Square).spanned(close));
}

#[derive(Debug, Copy, Clone)]
pub struct DelimitedShape;

impl ColorSyntax for DelimitedShape {
    type Info = ();
    type Input = (Delimiter, Span, Span);
    fn color_syntax<'a, 'b>(
        &self,
        (delimiter, open, close): &(Delimiter, Span, Span),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
        shapes: &mut Vec<Spanned<FlatShape>>,
    ) -> Self::Info {
        shapes.push(FlatShape::OpenDelimiter(*delimiter).spanned(*open));
        color_syntax(&ExpressionListShape, token_nodes, context, shapes);
        shapes.push(FlatShape::CloseDelimiter(*delimiter).spanned(*close));
    }
}
