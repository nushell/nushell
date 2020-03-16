use crate::hir::{
    self, syntax_shape::ExpandSyntax, syntax_shape::FlatShape, syntax_shape::NumberExpressionShape,
    syntax_shape::StringShape,
};
use crate::hir::{Expression, TokensIterator};
use crate::parse::token_tree::SeparatorType;

use nu_errors::ParseError;
use nu_protocol::UntaggedValue;
use nu_source::Span;

#[derive(Debug, Copy, Clone)]
pub struct LineSeparatedShape;

impl ExpandSyntax for LineSeparatedShape {
    type Output = Result<Vec<UntaggedValue>, ParseError>;

    fn name(&self) -> &'static str {
        "any string line separated by"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
    ) -> Result<Vec<UntaggedValue>, ParseError> {
        let source = token_nodes.source();

        if token_nodes.at_end() {
            return Ok(vec![]);
        }

        let mut entries = vec![];

        loop {
            let field = {
                token_nodes
                    .expand_syntax(NumberExpressionShape)
                    .or_else(|_| {
                        token_nodes
                            .expand_syntax(StringShape)
                            .map(|syntax| Expression::string(syntax.inner).into_expr(syntax.span))
                    })
            };

            if let Ok(field) = field {
                match &field.expr {
                    Expression::Literal(hir::Literal::Number(crate::Number::Int(i))) => {
                        entries.push(UntaggedValue::int(i.clone()))
                    }
                    Expression::Literal(hir::Literal::Number(crate::Number::Decimal(d))) => {
                        entries.push(UntaggedValue::decimal(d.clone()))
                    }
                    Expression::Literal(hir::Literal::String(span)) => {
                        if span.is_closed() {
                            entries.push(UntaggedValue::nothing())
                        } else {
                            entries.push(UntaggedValue::string(span.slice(&source)))
                        }
                    }
                    _ => {}
                }
            }

            match token_nodes.expand_infallible(SeparatorShape) {
                Err(err) if !token_nodes.at_end() => return Err(err),
                _ => {}
            }

            if token_nodes.at_end() {
                break;
            }
        }

        Ok(entries)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct SeparatorShape;

impl ExpandSyntax for SeparatorShape {
    type Output = Result<Span, ParseError>;

    fn name(&self) -> &'static str {
        "separated"
    }

    fn expand<'a, 'b>(&self, token_nodes: &'b mut TokensIterator<'a>) -> Result<Span, ParseError> {
        token_nodes.expand_token(SeparatorType, |span| Ok((FlatShape::Separator, span)))
    }
}
