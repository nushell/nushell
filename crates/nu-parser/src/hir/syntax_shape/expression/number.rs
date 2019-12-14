use crate::hir::syntax_shape::{ExpandSyntax, FlatShape, VariableShape};
use crate::hir::{Expression, SpannedExpression};
use crate::hir::{RawNumber, TokensIterator};
use crate::parse::token_tree::{DecimalType, IntType};
use nu_errors::ParseError;
use nu_source::HasSpan;

#[derive(Debug, Copy, Clone)]
pub struct NumberExpressionShape;

impl ExpandSyntax for NumberExpressionShape {
    type Output = Result<SpannedExpression, ParseError>;

    fn name(&self) -> &'static str {
        "number"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
    ) -> Result<SpannedExpression, ParseError> {
        let source = token_nodes.source();

        token_nodes
            .expand_syntax(NumberShape)
            .map(|number| Expression::number(number.to_number(&source)).into_expr(number.span()))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct IntExpressionShape;

impl ExpandSyntax for IntExpressionShape {
    type Output = Result<SpannedExpression, ParseError>;

    fn name(&self) -> &'static str {
        "integer"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
    ) -> Result<SpannedExpression, ParseError> {
        let source = token_nodes.source();

        token_nodes.expand_syntax(VariableShape).or_else(|_| {
            token_nodes.expand_token(IntType, |number| {
                Ok((
                    FlatShape::Int,
                    Expression::number(number.to_number(&source)),
                ))
            })
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub struct IntShape;

impl ExpandSyntax for IntShape {
    type Output = Result<RawNumber, ParseError>;

    fn name(&self) -> &'static str {
        "integer"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
    ) -> Result<RawNumber, ParseError> {
        token_nodes.expand_token(IntType, |number| Ok((FlatShape::Int, number)))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct DecimalShape;

impl ExpandSyntax for DecimalShape {
    type Output = Result<RawNumber, ParseError>;

    fn name(&self) -> &'static str {
        "decimal"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
    ) -> Result<RawNumber, ParseError> {
        token_nodes.expand_token(DecimalType, |number| Ok((FlatShape::Decimal, number)))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct NumberShape;

impl ExpandSyntax for NumberShape {
    type Output = Result<RawNumber, ParseError>;

    fn name(&self) -> &'static str {
        "decimal"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
    ) -> Result<RawNumber, ParseError> {
        token_nodes
            .expand_syntax(IntShape)
            .or_else(|_| token_nodes.expand_syntax(DecimalShape))
    }
}
