use crate::hir::syntax_shape::{
    expression::expand_file_path, BarePathShape, DecimalShape, ExpandContext, ExpandSyntax,
    FlatShape, IntShape, StringShape, VariablePathShape,
};
use crate::hir::{Expression, SpannedExpression, TokensIterator};
use crate::parse::token_tree::ExternalWordType;
use nu_errors::ParseError;
use nu_source::{HasSpan, Span};

#[derive(Debug, Copy, Clone)]
pub struct FilePathShape;

impl ExpandSyntax for FilePathShape {
    type Output = Result<SpannedExpression, ParseError>;

    fn name(&self) -> &'static str {
        "file path"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
    ) -> Result<SpannedExpression, ParseError> {
        token_nodes
            .expand_syntax(VariablePathShape)
            .or_else(|_| {
                token_nodes
                    .expand_syntax(BarePathShape)
                    .or_else(|_| token_nodes.expand_syntax(ExternalWordShape))
                    .map(|span| file_path(span, token_nodes.context()).into_expr(span))
            })
            .or_else(|_| {
                token_nodes.expand_syntax(StringShape).map(|syntax| {
                    file_path(syntax.inner, token_nodes.context()).into_expr(syntax.span)
                })
            })
            .or_else(|_| {
                token_nodes
                    .expand_syntax(IntShape)
                    .or_else(|_| token_nodes.expand_syntax(DecimalShape))
                    .map(|number| {
                        file_path(number.span(), token_nodes.context()).into_expr(number.span())
                    })
            })
            .map_err(|_| token_nodes.err_next_token("file path"))
    }
}

fn file_path(text: Span, context: &ExpandContext) -> Expression {
    Expression::FilePath(expand_file_path(text.slice(context.source), context))
}

#[derive(Debug, Copy, Clone)]
pub struct ExternalWordShape;

impl ExpandSyntax for ExternalWordShape {
    type Output = Result<Span, ParseError>;

    fn name(&self) -> &'static str {
        "external word"
    }

    fn expand<'a, 'b>(&self, token_nodes: &'b mut TokensIterator<'a>) -> Result<Span, ParseError> {
        token_nodes.expand_token(ExternalWordType, |span| Ok((FlatShape::ExternalWord, span)))
    }
}
