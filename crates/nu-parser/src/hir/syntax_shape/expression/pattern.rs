use crate::hir::syntax_shape::{
    expand_bare, expression::expand_file_path, BarePathShape, ExpandContext, ExpandSyntax,
    ExternalWordShape, StringShape,
};
use crate::hir::{Expression, SpannedExpression};
use crate::parse::operator::EvaluationOperator;
use crate::{hir, hir::TokensIterator, Token};
use nu_errors::ParseError;
use nu_source::Span;

#[derive(Debug, Copy, Clone)]
pub struct PatternShape;

impl ExpandSyntax for PatternShape {
    type Output = Result<SpannedExpression, ParseError>;

    fn name(&self) -> &'static str {
        "glob pattern"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
    ) -> Result<hir::SpannedExpression, ParseError> {
        let (inner, outer) = token_nodes
            .expand_syntax(BarePatternShape)
            .or_else(|_| token_nodes.expand_syntax(BarePathShape))
            .or_else(|_| token_nodes.expand_syntax(ExternalWordShape))
            .map(|span| (span, span))
            .or_else(|_| {
                token_nodes
                    .expand_syntax(StringShape)
                    .map(|syntax| (syntax.inner, syntax.span))
            })
            .map_err(|_| token_nodes.err_next_token("glob pattern"))?;

        Ok(file_pattern(inner, outer, token_nodes.context()))
    }
}

fn file_pattern(body: Span, outer: Span, context: &ExpandContext) -> SpannedExpression {
    let path = expand_file_path(body.slice(context.source), context);
    Expression::pattern(path.to_string_lossy()).into_expr(outer)
}

#[derive(Debug, Copy, Clone)]
pub struct PatternExpressionShape;

impl ExpandSyntax for PatternExpressionShape {
    type Output = Result<SpannedExpression, ParseError>;

    fn name(&self) -> &'static str {
        "pattern"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
    ) -> Result<SpannedExpression, ParseError> {
        token_nodes.expand_syntax(BarePatternShape).map(|span| {
            let path = expand_file_path(span.slice(&token_nodes.source()), token_nodes.context());
            Expression::pattern(path.to_string_lossy()).into_expr(span)
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub struct BarePatternShape;

impl ExpandSyntax for BarePatternShape {
    type Output = Result<Span, ParseError>;

    fn name(&self) -> &'static str {
        "bare pattern"
    }

    fn expand<'a, 'b>(&self, token_nodes: &'b mut TokensIterator<'a>) -> Result<Span, ParseError> {
        expand_bare(token_nodes, |token| match token.unspanned() {
            Token::Bare
            | Token::EvaluationOperator(EvaluationOperator::Dot)
            | Token::GlobPattern => true,

            _ => false,
        })
    }
}
