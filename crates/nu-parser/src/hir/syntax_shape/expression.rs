pub(crate) mod delimited;
pub(crate) mod file_path;
pub(crate) mod list;
pub(crate) mod number;
pub(crate) mod pattern;
pub(crate) mod range;
pub(crate) mod string;
pub(crate) mod unit;
pub(crate) mod variable_path;

use crate::hir::syntax_shape::{
    BareExpressionShape, DelimitedSquareShape, ExpandContext, ExpandSyntax,
    ExpressionContinuationShape, NumberExpressionShape, PatternExpressionShape,
    StringExpressionShape, UnitExpressionShape, VariableShape,
};
use crate::hir::{SpannedExpression, TokensIterator};
use nu_errors::ParseError;
use std::path::PathBuf;

#[derive(Debug, Copy, Clone)]
pub struct AnyExpressionShape;

impl ExpandSyntax for AnyExpressionShape {
    type Output = Result<SpannedExpression, ParseError>;

    fn name(&self) -> &'static str {
        "any expression"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
    ) -> Result<SpannedExpression, ParseError> {
        token_nodes.atomic_parse(|token_nodes| {
            // Look for an atomic expression at the cursor
            let mut current = token_nodes.expand_syntax(AnyExpressionStartShape)?;

            loop {
                match token_nodes.expand_syntax(ExpressionContinuationShape) {
                    Err(_) => return Ok(current),
                    Ok(continuation) => current = continuation.append_to(current),
                }
            }
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub struct AnyExpressionStartShape;

impl ExpandSyntax for AnyExpressionStartShape {
    type Output = Result<SpannedExpression, ParseError>;

    fn name(&self) -> &'static str {
        "any expression start"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
    ) -> Result<SpannedExpression, ParseError> {
        token_nodes
            .expand_syntax(VariableShape)
            .or_else(|_| token_nodes.expand_syntax(UnitExpressionShape))
            .or_else(|_| token_nodes.expand_syntax(BareExpressionShape))
            .or_else(|_| token_nodes.expand_syntax(PatternExpressionShape))
            .or_else(|_| token_nodes.expand_syntax(NumberExpressionShape))
            .or_else(|_| token_nodes.expand_syntax(StringExpressionShape))
            .or_else(|_| token_nodes.expand_syntax(DelimitedSquareShape))
    }
}

pub fn expand_file_path(string: &str, context: &ExpandContext) -> PathBuf {
    let expanded = shellexpand::tilde_with_context(string, || context.homedir());

    PathBuf::from(expanded.as_ref())
}
