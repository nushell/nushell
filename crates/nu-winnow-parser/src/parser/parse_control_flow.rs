//! Control flow: `if`, `match`, `while`, `loop`, `try`, `return`, `break`
//! and `continue`.
//!
//! nu-parser has no file for these: in nu they are ordinary commands whose
//! signatures take blocks and keywords, parsed by `parse_call`. Here each is a
//! node of its own, parsed from its fixed shape with a [`KeywordCall`], which
//! gives them nu's handling of flags.

use winnow::Parser;
use winnow::combinator::{alt, opt};

use crate::ast::{Else, Expr, Expression, Handler, HandlerKind, If, Loop, Match, Return, Try, While};
use crate::error::{Diagnostic, ErrorKind};
use crate::input::{ParseResult, cut};
use crate::lex::Token;

use super::WorkingSet;
use super::parse_expressions::{
    BraceShape, ExpectedShape, Position, brace_shape, parse_block_body, parse_brace_record, parse_closure_expression,
    parse_expression, parse_match_block_expression, parse_math_expression, parse_value,
};
use super::parse_helpers::is_help_flag;
use super::parse_keywords::{KeywordCall, parse_block_or_value_argument, parse_brace_value};
use super::tokens::{Tokens, expected, keyword, tokens_until};

/// A `{ ... }` where nu accepts a block or any expression (the `else` branch
/// and match arms): a closure or a record parses as that value, the rest is
/// a block. An item with a tail is a value or never closes, as nu finds when
/// it falls back to an expression (`else {b: 2}.b`, `else {}.b`).
fn parse_block_or_value<'a>(working_set: &WorkingSet<'a>, token: &Token) -> ParseResult<Expression<'a>> {
    if !working_set.get_span_contents(token.span).ends_with('}') {
        return parse_value(working_set, token.span, ExpectedShape::Any);
    }
    match brace_shape(working_set, token.span)? {
        BraceShape::ClosureParams | BraceShape::Record => parse_value(working_set, token.span, ExpectedShape::Any),
        BraceShape::Empty | BraceShape::Spread | BraceShape::Other => {
            Ok(Expression::new(Expr::Block(parse_block_body(working_set, token.span)?), token.span))
        }
    }
}

/// `if condition... { block } [else { block } | else expression...]`: the
/// condition is every item before the block, which is the item before the
/// `else`, or the last item.
pub fn parse_if<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    let mut call = KeywordCall::start(&mut tokens)?;
    call.flags(&mut tokens)?;
    if call.wants_help() && tokens.at_end() {
        return call.help_call();
    }
    let then_part = tokens_until("else").parse_next(&mut tokens)?;
    let else_keyword = opt(keyword("else")).parse_next(&mut tokens)?;
    let (condition, block) = match (then_part.all(), else_keyword) {
        // nu's math-expression condition leaves the last item for the block,
        // where `--help` is a flag of `if`: `if x --help` shows help, once the
        // condition, every item before it, parses (`if if --help` is still an
        // error, `if if --help --help` is not).
        ([condition @ .., last], None) if !condition.is_empty() && is_help_flag(working_set, last) => {
            parse_math_expression(then_part.slice(0..condition.len()))?;
            return call.help_call();
        }
        ([], Some(else_keyword)) => {
            return Err(cut(Diagnostic::expected("condition and block before `else`", else_keyword.span)));
        }
        ([condition @ .., block], _) if !condition.is_empty() => (then_part.slice(0..condition.len()), block),
        // `if --help x`: help, once the condition parses (`if --help if` fails).
        (items, _) if call.wants_help() => {
            if !items.is_empty() {
                parse_math_expression(then_part)?;
            }
            return call.help_call();
        }
        (items, _) => {
            let at = items.first().map_or(call.keyword.span.past(), |token| token.span);
            return Err(cut(Diagnostic::expected("condition", at)));
        }
    };
    let condition = parse_math_expression(condition)?;
    let (then_block, then_value) = parse_block_or_value_argument(working_set, block, "block after the condition")?;
    let mut span = call.keyword.span.merge(block.span);
    let else_branch = match else_keyword {
        None => None,
        Some(else_keyword) => {
            let body = match tokens.remaining() {
                [] => {
                    return Err(cut(Diagnostic::expected(
                        "block or expression after `else`",
                        else_keyword.span.past(),
                    )));
                }
                [only] if tokens.text(only).starts_with('{') => parse_block_or_value(working_set, only)?,
                _ => parse_expression(tokens.rest_stream(), Position::Element)?,
            };
            span = span.merge(body.span);
            Some(Else { keyword: else_keyword.span, body: Box::new(body) })
        }
    };
    let if_expression = If { condition: Box::new(condition), then_block, then_value, else_branch };
    call.finish(Expression::new(Expr::If(if_expression), span))
}

/// `match value { pattern => body, ... }`.
pub fn parse_match<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    let mut call = KeywordCall::start(&mut tokens)?;
    let Some(value) = call.positional(&mut tokens, "value to match on")? else { return call.help_call() };
    let value = parse_value(working_set, value.span, ExpectedShape::Any)?;
    let Some(block) = call.positional(&mut tokens, "match block")? else { return call.help_call() };
    // nu decides what the `{ ... }` is before it knows it wants arms: a
    // `key:` makes it a record or a cell path on one (`{a: 1}.a`), closure
    // parameters a closure, and it accepts either; a variable or a
    // subexpression in that position is accepted as well.
    let (arms, value_block) = match tokens.text(&block).as_bytes().first() {
        Some(b'{') => match parse_brace_record(working_set, block.span)? {
            Some(value) => (Vec::new(), Some(Box::new(value))),
            None => match brace_shape(working_set, block.span)? {
                BraceShape::ClosureParams => {
                    (Vec::new(), Some(Box::new(parse_value(working_set, block.span, ExpectedShape::Any)?)))
                }
                _ => (parse_match_block_expression(working_set, block.span)?, None),
            },
        },
        Some(b'$' | b'(') => (Vec::new(), Some(Box::new(parse_value(working_set, block.span, ExpectedShape::Any)?))),
        _ => return Err(cut(Diagnostic::expected("match block", block.span))),
    };
    call.end(&mut tokens)?;
    let match_expression = Match { value: Box::new(value), block_span: block.span, arms, value_block };
    let span = call.keyword.span.merge(block.span);
    call.finish(Expression::new(Expr::Match(match_expression), span))
}

/// `while condition... { block }`: the condition is every item before the
/// last, which is the block.
pub fn parse_while<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let mut call = KeywordCall::start(&mut tokens)?;
    call.flags(&mut tokens)?;
    // nu's math-expression condition leaves the last item for the block,
    // where `--help` is a flag of `while`: `while x --help` shows help, once
    // the condition, every item before it, parses (`while if --help` is still
    // an error).
    let start = tokens.position();
    if let [condition @ .., last] = tokens.remaining()
        && !condition.is_empty()
        && is_help_flag(tokens.working_set, last)
    {
        parse_math_expression(tokens.slice(start..start + condition.len()))?;
        return call.help_call();
    }
    let Some((block, condition)) = tokens.remaining().split_last().filter(|(_, condition)| !condition.is_empty())
    else {
        if call.wants_help() {
            // `while --help x`: help, once the condition parses.
            if !tokens.at_end() {
                parse_math_expression(tokens.slice(start..tokens.all().len()))?;
            }
            return call.help_call();
        }
        return Err(cut(Diagnostic::expected("condition and block", tokens.end_span())));
    };
    let condition = parse_math_expression(tokens.slice(start..start + condition.len()))?;
    let (body, body_value) = parse_block_or_value_argument(tokens.working_set, block, "block")?;
    let span = call.keyword.span.merge(block.span);
    call.finish(Expression::new(Expr::While(While { condition: Box::new(condition), body, body_value }), span))
}

/// `loop { block }`.
pub fn parse_loop<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let mut call = KeywordCall::start(&mut tokens)?;
    let Some(block) = call.positional(&mut tokens, "block")? else { return call.help_call() };
    let (body, body_value) = parse_block_or_value_argument(tokens.working_set, &block, "block")?;
    call.end(&mut tokens)?;
    let span = call.keyword.span.merge(block.span);
    call.finish(Expression::new(Expr::Loop(Loop { body, body_value }), span))
}

/// `try { block } [catch handler] [finally handler]`, the handlers in either
/// order, at most two.
pub fn parse_try<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    let mut call = KeywordCall::start(&mut tokens)?;
    let Some(block) = call.positional(&mut tokens, "block")? else { return call.help_call() };
    let (body, body_value) = parse_block_or_value_argument(working_set, &block, "block")?;
    let mut span = call.keyword.span.merge(block.span);
    let mut handlers = Vec::new();
    loop {
        // `try {} --`: the marker may be the last item.
        call.flags(&mut tokens)?;
        if tokens.at_end() {
            break;
        }
        let handler_keyword =
            expected("`catch` or `finally`", alt((keyword("catch"), keyword("finally")))).parse_next(&mut tokens)?;
        if handlers.len() == 2 {
            return Err(cut(Diagnostic::new(ErrorKind::ExtraTokens, handler_keyword.span)
                .with_help("`try` takes at most two handlers (`catch` and `finally`)")));
        }
        let kind = match tokens.text(&handler_keyword) {
            "catch" => HandlerKind::Catch,
            _ => HandlerKind::Finally,
        };
        let handler = tokens.expect_item("closure")?;
        span = span.merge(handler.span);
        let body = Box::new(parse_try_handler(working_set, &handler)?);
        handlers.push(Handler { kind, keyword: handler_keyword.span, body });
    }
    call.finish(Expression::new(Expr::Try(Try { body, body_value, handlers }), span))
}

/// A `catch`/`finally` handler: a closure, or a variable, subexpression or
/// brace value ([`parse_brace_value`]) that may hold one.
fn parse_try_handler<'a>(working_set: &WorkingSet<'a>, token: &Token) -> ParseResult<Expression<'a>> {
    match working_set.get_span_contents(token.span).as_bytes().first() {
        Some(b'{') => match parse_brace_value(working_set, token.span, "closure")? {
            Some(value) => Ok(value),
            None => parse_closure_expression(working_set, token.span),
        },
        Some(b'$' | b'(') => parse_value(working_set, token.span, ExpectedShape::Any),
        _ => Err(cut(Diagnostic::expected("closure", token.span))),
    }
}

/// `return [value]`.
pub fn parse_return<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let mut call = KeywordCall::start(&mut tokens)?;
    call.flags(&mut tokens)?;
    let value = match tokens.at_end() {
        true => None,
        false => {
            let value = tokens.expect_item("value")?;
            Some(Box::new(parse_value(tokens.working_set, value.span, ExpectedShape::Any)?))
        }
    };
    call.end(&mut tokens)?;
    let span = value.as_ref().map_or(call.keyword.span, |value| call.keyword.span.merge(value.span));
    call.finish(Expression::new(Expr::Return(Return { value }), span))
}

/// `break` or `continue`, which take no arguments.
pub fn parse_break_or_continue<'a>(mut tokens: Tokens<'_, 'a>, expr: Expr<'a>) -> ParseResult<Expression<'a>> {
    let mut call = KeywordCall::start(&mut tokens)?;
    call.end(&mut tokens)?;
    let span = call.keyword.span;
    call.finish(Expression::new(expr, span))
}
