use crate::hir::syntax_shape::{
    color_syntax, expand_syntax, ColorSyntax, ExpandContext, ExpressionListShape, TokenNode,
};
use crate::{hir, hir::TokensIterator, Delimiter, FlatShape};
use nu_errors::ParseError;
use nu_source::{Span, SpannedItem, Tag};

pub fn expand_delimited_square(
    children: &[TokenNode],
    span: Span,
    context: &ExpandContext,
) -> Result<hir::Expression, ParseError> {
    let mut tokens = TokensIterator::new(&children, span, context.source.clone(), false);

    let list = expand_syntax(&ExpressionListShape, &mut tokens, context);

    Ok(hir::Expression::list(
        list?.exprs.item,
        Tag { span, anchor: None },
    ))
}

pub fn color_delimited_square(
    (open, close): (Span, Span),
    token_nodes: &mut TokensIterator,
    _span: Span,
    context: &ExpandContext,
) {
    token_nodes.color_shape(FlatShape::OpenDelimiter(Delimiter::Square).spanned(open));
    let _list = color_syntax(&ExpressionListShape, token_nodes, context);
    token_nodes.color_shape(FlatShape::CloseDelimiter(Delimiter::Square).spanned(close));
}

#[derive(Debug, Copy, Clone)]
pub struct DelimitedShape;

impl ColorSyntax for DelimitedShape {
    type Info = ();
    type Input = (Delimiter, Span, Span);

    fn name(&self) -> &'static str {
        "DelimitedShape"
    }

    fn color_syntax<'a, 'b>(
        &self,
        (delimiter, open, close): &(Delimiter, Span, Span),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Self::Info {
        token_nodes.color_shape(FlatShape::OpenDelimiter(*delimiter).spanned(*open));
        color_syntax(&ExpressionListShape, token_nodes, context);
        token_nodes.color_shape(FlatShape::CloseDelimiter(*delimiter).spanned(*close));
    }
}
