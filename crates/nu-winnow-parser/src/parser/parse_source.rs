//! `where` (nu-parser keeps `parse_where` in `parse_source.rs`).

use crate::ast::{Expr, Expression, Where};
use crate::error::Diagnostic;
use crate::input::{ParseResult, cut};

use super::parse_expressions::{parse_closure_expression, parse_row_condition};
use super::parse_keywords::KeywordCall;
use super::tokens::Tokens;

/// `where { closure }` or `where row-condition...`.
pub fn parse_where<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let mut call = KeywordCall::start(&mut tokens)?;
    call.flags(&mut tokens)?;
    let condition = match tokens.remaining() {
        [] if call.wants_help() => return call.help_call(),
        [] => return Err(cut(Diagnostic::expected("row condition or closure", call.keyword.span.past()))),
        _ => parse_condition(tokens)?,
    };
    let span = call.keyword.span.merge(condition.span);
    call.finish(Expression::new(Expr::Where(Where { condition: Box::new(condition) }), span))
}

/// The condition of `where`, `any`, `take while`, ...: the items left in `tokens`, one or more
/// (nu's `RowCondition` shape). nu tries a closure first and falls back to a row condition,
/// which takes a brace item with a tail as a value (`where {}..`).
pub fn parse_condition<'a>(tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    match tokens.remaining() {
        [only] if tokens.text(only).starts_with('{') && tokens.text(only).ends_with('}') => {
            parse_closure_expression(tokens.working_set, only.span)
        }
        _ => parse_row_condition(tokens.rest_stream()),
    }
}
