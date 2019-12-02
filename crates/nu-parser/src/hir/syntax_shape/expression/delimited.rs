use crate::hir::syntax_shape::{
    color_syntax, expand_syntax, ColorSyntax, ExpandContext, ExpressionListShape, TokenNode,
};
use crate::{hir, hir::TokensIterator, Delimiter, FlatShape};
use nu_errors::ParseError;
#[cfg(not(coloring_in_tokens))]
use nu_source::Spanned;
use nu_source::{Span, SpannedItem, Tag};

pub fn expand_delimited_square(
    children: &Vec<TokenNode>,
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

#[cfg(not(coloring_in_tokens))]
pub fn color_delimited_square(
    (open, close): (Span, Span),
    children: &Vec<TokenNode>,
    span: Span,
    context: &ExpandContext,
    shapes: &mut Vec<Spanned<FlatShape>>,
) {
    shapes.push(FlatShape::OpenDelimiter(Delimiter::Square).spanned(open));
    let mut tokens = TokensIterator::new(&children, span, context.source.clone(), false);
    let _list = color_syntax(&ExpressionListShape, &mut tokens, context, shapes);
    shapes.push(FlatShape::CloseDelimiter(Delimiter::Square).spanned(close));
}

#[cfg(coloring_in_tokens)]
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

#[cfg(not(coloring_in_tokens))]
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

#[cfg(coloring_in_tokens)]
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
