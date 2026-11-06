//! `where` (nu-parser keeps `parse_where` in `parse_source.rs`).

use crate::winnow::syntax::ast::{Expr, Expression, Where};
use crate::winnow::syntax::input::ParseResult;

use super::parse_expressions::{parse_closure_expression, parse_row_condition, recover_literal};
use super::parse_keywords::KeywordCall;
use super::tokens::Tokens;

/// `where { closure }` or `where row-condition...`.
pub fn parse_where<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let mut call = KeywordCall::start(&mut tokens)?;
    if call.call_flags(&mut tokens)? {
        return call.help_call();
    }
    let condition = match tokens.remaining() {
        [] => {
            call.missing();
            return call.help_call();
        }
        _ => parse_condition(tokens)?,
    };
    let span = call.keyword.span.merge(condition.span);
    call.finish(Expression::new(
        Expr::Where(Where {
            condition: Box::new(condition),
        }),
        span,
    ))
}

/// The condition of `where`, `any`, `take while`, ...: the items left in `tokens`, one or more
/// (nu's `RowCondition` shape). nu tries a closure first and falls back to a row condition,
/// which takes a brace item with a tail as a value (`where {}..`).
pub fn parse_condition<'a>(tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    match tokens.remaining() {
        // A closure with an error the parser goes on past is a garbage node, which the
        // consumer reads again with nu's error (`where {|x 1}`).
        [only] if tokens.text(only).starts_with('{') && tokens.text(only).ends_with('}') => {
            let closure = parse_closure_expression(tokens.working_set, only.span);
            recover_literal(tokens.working_set, only.span, closure)
        }
        _ => parse_row_condition(tokens.rest_stream()),
    }
}
