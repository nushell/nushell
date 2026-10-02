//! `where` (nu-parser keeps `parse_where` in `parse_source.rs`).

use crate::ast::{Expr, Expression, Where};
use crate::error::Diagnostic;
use crate::input::{ParseResult, cut};

use super::parse_expressions::{parse_closure_expression, parse_row_condition};
use super::parse_keywords::KeywordCall;
use super::tokens::Tokens;

/// `where { closure }` or `where row-condition...`.
pub fn parse_where<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    let mut call = KeywordCall::start(&mut tokens)?;
    call.flags(&mut tokens)?;
    let condition = match tokens.remaining() {
        [] if call.wants_help() => return call.help_call(),
        [] => return Err(cut(Diagnostic::expected("row condition or closure", call.keyword.span.past()))),
        // nu tries a closure first and falls back to a row condition, which
        // takes a brace item with a tail as a value (`where {}..`).
        [only] if tokens.text(only).starts_with('{') && tokens.text(only).ends_with('}') => {
            parse_closure_expression(working_set, only.span)?
        }
        _ => parse_row_condition(tokens.rest_stream())?,
    };
    let span = call.keyword.span.merge(condition.span);
    call.finish(Expression::new(Expr::Where(Where { condition: Box::new(condition) }), span))
}
