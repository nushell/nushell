//! Control flow: `if`, `match`, `while`, `loop`, `try`, `return`, `break`
//! and `continue`.
//!
//! nu-parser has no file for these: in nu they are ordinary commands whose
//! signatures take blocks and keywords, parsed by `parse_call`. Here each is a
//! node of its own, parsed from its fixed shape with a [`KeywordCall`], which
//! gives them nu's handling of flags.

use winnow::Parser;
use winnow::combinator::{alt, opt};
use winnow::error::ErrMode;

use crate::winnow::syntax::ast::{
    Else, Expr, Expression, Handler, HandlerKind, If, Loop, Match, MatchArm, Return, Try, While,
};
use crate::winnow::syntax::error::Diagnostic;
use crate::winnow::syntax::input::{ParseFailure, ParseResult, cut};
use crate::winnow::syntax::lex::Token;

use super::WorkingSet;
use super::parse_expressions::{
    BraceShape, ExpectedShape, Position, brace_shape, parse_block_body, parse_brace_record,
    parse_closure_expression, parse_expression, parse_match_block_expression,
    parse_math_expression, parse_value,
};
use super::parse_helpers::is_help_flag;
use super::parse_keywords::{
    KeywordCall, call_after_error, parse_block_or_value_argument, parse_brace_value,
    parse_help_call, seen_end_of_options,
};
use super::tokens::{Tokens, expected, keyword, tokens_until};

/// A `{ ... }` where nu accepts a block or any expression (the `else` branch
/// and match arms): a closure or a record parses as that value, the rest is
/// a block. An item with a tail is a value or never closes, as nu finds when
/// it falls back to an expression (`else {b: 2}.b`, `else {}.b`).
fn parse_block_or_value<'a>(
    working_set: &WorkingSet<'a>,
    token: &Token,
) -> ParseResult<Expression<'a>> {
    if !working_set.get_span_contents(token.span).ends_with('}') {
        return parse_value(working_set, token.span, ExpectedShape::Any);
    }
    match brace_shape(working_set, token.span)? {
        BraceShape::ClosureParams | BraceShape::Record => {
            parse_value(working_set, token.span, ExpectedShape::Any)
        }
        BraceShape::Empty | BraceShape::Spread | BraceShape::Other => Ok(Expression::new(
            Expr::Block(parse_block_body(working_set, token.span)?),
            token.span,
        )),
    }
}

/// `if condition... { block } [else { block } | else expression...]`: the
/// condition is every item before the block, which is the item before the
/// `else`, or the last item.
///
/// `else if ...` needs no rule of its own: the expression after `else` is
/// another `if`, which takes the rest of the items, its own `else` included.
///
/// nu's call to `if` reports an error in its condition or its block (`if true 1`, `if $a
/// and`, `if --help true {} x`) and goes on, so the statement is then that call, the error
/// recorded as recovered ([`call_after_error`]).
pub fn parse_if<'a>(tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let statement = tokens;
    parse_if_parts(tokens).or_else(|error| call_after_error(statement, error, parse_help_call))
}

fn parse_if_parts<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    let mut call = KeywordCall::start(&mut tokens)?;
    if let Some(garbage) = lone_keyword(&tokens, &call, "if") {
        return Ok(garbage);
    }
    if call.call_flags(&mut tokens)? {
        return call.help_call();
    }
    if call.is_call() && tokens.at_end() {
        return call.help_call();
    }
    let options_ended = seen_end_of_options(&tokens);
    let then_part = tokens_until("else").parse_next(&mut tokens)?;
    let else_keyword = opt(keyword("else")).parse_next(&mut tokens)?;
    let (condition, block) = match (then_part.all(), else_keyword) {
        // nu's math-expression condition leaves the last item for the block,
        // where `--help` is a flag of `if`: `if x --help` shows help, once the
        // condition, every item before it, parses (`if if --help` is still an
        // error, `if if --help --help` is not).
        ([condition @ .., last], None)
            if !condition.is_empty() && is_help_flag(working_set, last) =>
        {
            parse_math_expression(then_part.slice(0..condition.len()))?;
            return call.help_call();
        }
        // A `--` there ends the options, and the block is missing (`if true --`).
        ([condition @ .., last], None)
            if !condition.is_empty() && tokens.text(last) == "--" && !options_ended =>
        {
            parse_math_expression(then_part.slice(0..condition.len()))?;
            call.missing();
            return call.help_call();
        }
        // `if else {}`: nu's call misses the condition, `else` being a later keyword.
        ([], Some(_)) => {
            call.missing();
            return call.help_call();
        }
        ([condition @ .., block], _) if !condition.is_empty() => {
            (then_part.slice(0..condition.len()), block)
        }
        // A condition without a block (`if true`), or nothing: nu's call misses a positional.
        // `if --help x` shows help. Either once the condition parses (`if --help if` fails).
        (items, _) => {
            if !items.is_empty() {
                parse_math_expression(then_part)?;
            }
            call.missing();
            return call.help_call();
        }
    };
    let condition = parse_math_expression(condition)?;
    let (then_block, then_value) =
        parse_block_or_value_argument(working_set, block, "block after the condition")?;
    let mut span = call.keyword.span.merge(block.span);
    let else_branch = match else_keyword {
        None => None,
        Some(else_keyword) => {
            let body = match tokens.remaining() {
                // nu's KeywordMissingArgument, which it reports and goes on past.
                [] => {
                    return call.call_after(Diagnostic::expected(
                        "block or expression after `else`",
                        else_keyword.span.past(),
                    ));
                }
                // nu's `else` argument is a block or an expression, a block first: a `{ ... }`
                // is the argument alone, leaving the items after it to `if`, flags
                // (`if c {} else {} --help`) or extra arguments (`else {} * 3`, an error). Only
                // braces with parameters, which no block takes, start an expression of the
                // rest when more than flags follow (`else {|x| $x} == $f`).
                [first, rest @ ..]
                    if tokens.text(first).starts_with('{')
                        && (rest.iter().all(|token| tokens.text(token).starts_with('-'))
                            || !matches!(
                                brace_shape(working_set, first.span),
                                Ok(BraceShape::ClosureParams)
                            )) =>
                {
                    let body = parse_block_or_value(working_set, first)?;
                    tokens.next_token();
                    call.end(&mut tokens)?;
                    body
                }
                _ => parse_expression(tokens.rest_stream(), Position::Element)?,
            };
            span = span.merge(body.span);
            Some(Else {
                keyword: else_keyword.span,
                body: Box::new(body),
            })
        }
    };
    let if_expression = If {
        keyword: call.keyword.span,
        condition: Box::new(condition),
        then_block,
        then_value,
        else_branch,
    };
    call.finish(Expression::new(Expr::If(if_expression), span))
}

/// A statement that is `keyword` alone (`if`, `match`), not a call through an alias of it: nu
/// reads it as a math expression (`is_math_expression_like`) missing its operand, an error it
/// goes on past with garbage (`parse_math_expression`), so the error is recorded as recovered
/// and the statement is garbage, which the consumer reads with nu's math parser.
fn lone_keyword<'a>(
    tokens: &Tokens<'_, 'a>,
    call: &KeywordCall<'_, 'a>,
    keyword: &str,
) -> Option<Expression<'a>> {
    if !tokens.at_end() || tokens.text(&call.keyword) != keyword {
        return None;
    }
    let span = call.keyword.span;
    tokens
        .working_set
        .error(Diagnostic::expected("expression", span.past()).as_recovered());
    Some(Expression::new(Expr::Garbage, span))
}

/// `match value { pattern => body, ... }`.
pub fn parse_match<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    let mut call = KeywordCall::start(&mut tokens)?;
    if let Some(garbage) = lone_keyword(&tokens, &call, "match") {
        return Ok(garbage);
    }
    let Some(value) = call.positional(&mut tokens, "value to match on")? else {
        return call.help_call();
    };
    let value = parse_value(working_set, value.span, ExpectedShape::Any)?;
    let Some(block) = call.positional(&mut tokens, "match block")? else {
        return call.help_call();
    };
    // nu decides what the `{ ... }` is before it knows it wants arms: a
    // `key:` makes it a record or a cell path on one (`{a: 1}.a`), closure
    // parameters a closure, and it accepts either; a variable or a
    // subexpression in that position is accepted as well.
    let (arms, value_block) = match tokens.text(&block).as_bytes().first() {
        // A brace item nu's value parser reads past an error in (`match 1 {}.a`) too.
        Some(b'{') => match parse_brace_record(working_set, block.span) {
            Err(ErrMode::Cut(ParseFailure::Error(diagnostic))) => {
                garbage_match_block(working_set, &block, *diagnostic)
            }
            Err(error) => return Err(error),
            Ok(Some(value)) => (Vec::new(), Some(Box::new(value))),
            Ok(None) => match brace_shape(working_set, block.span) {
                Err(ErrMode::Cut(ParseFailure::Error(diagnostic))) => {
                    garbage_match_block(working_set, &block, *diagnostic)
                }
                Err(error) => return Err(error),
                Ok(BraceShape::ClosureParams) => (
                    Vec::new(),
                    Some(Box::new(parse_value(
                        working_set,
                        block.span,
                        ExpectedShape::Any,
                    )?)),
                ),
                Ok(_) => match parse_match_block_expression(working_set, block.span) {
                    Ok(arms) => (arms, None),
                    Err(ErrMode::Cut(ParseFailure::Error(diagnostic))) => {
                        garbage_match_block(working_set, &block, *diagnostic)
                    }
                    Err(error) => return Err(error),
                },
            },
        },
        Some(b'$' | b'(') => (
            Vec::new(),
            Some(Box::new(parse_value(
                working_set,
                block.span,
                ExpectedShape::Any,
            )?)),
        ),
        _ => garbage_match_block(
            working_set,
            &block,
            Diagnostic::expected("match block", block.span),
        ),
    };
    call.end(&mut tokens)?;
    let match_expression = Match {
        keyword: call.keyword.span,
        value: Box::new(value),
        block_span: block.span,
        arms,
        value_block,
    };
    let span = call.keyword.span.merge(block.span);
    call.finish(Expression::new(Expr::Match(match_expression), span))
}

/// The arms of a `match` whose block nu's value parser reads past `diagnostic` in: an error in
/// the arms (a guard without an expression, a missing `=>`, `parse_match_block_expression`
/// goes on with the arms it reads) or an argument that is no block (`match 1 [a]`, garbage).
/// The error is recorded as recovered and the block is a garbage value, which the consumer
/// reads again with nu's value parser for the `match-block` shape.
fn garbage_match_block<'a>(
    working_set: &WorkingSet<'a>,
    block: &Token,
    diagnostic: Diagnostic,
) -> (Vec<MatchArm<'a>>, Option<Box<Expression<'a>>>) {
    working_set.error(diagnostic.as_recovered());
    let garbage = Expression::new(Expr::Garbage, block.span);
    (Vec::new(), Some(Box::new(garbage)))
}

/// `while condition... { block }`: the condition is every item before the
/// last, which is the block. An error in either is nu's call's, as for `if` ([`parse_if`]).
pub fn parse_while<'a>(tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let statement = tokens;
    parse_while_parts(tokens).or_else(|error| call_after_error(statement, error, parse_help_call))
}

fn parse_while_parts<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let mut call = KeywordCall::start(&mut tokens)?;
    if call.call_flags(&mut tokens)? {
        return call.help_call();
    }
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
    // A `--` there ends the options, and the block is missing (`while true --`).
    if let [condition @ .., last] = tokens.remaining()
        && !condition.is_empty()
        && tokens.text(last) == "--"
        && !seen_end_of_options(&tokens)
    {
        parse_math_expression(tokens.slice(start..start + condition.len()))?;
        call.missing();
        return call.help_call();
    }
    let Some((block, condition)) = tokens
        .remaining()
        .split_last()
        .filter(|(_, condition)| !condition.is_empty())
    else {
        // A condition without a block (`while true`), or nothing: nu's call misses a
        // positional. `while --help x` shows help. Either once the condition parses.
        if !tokens.at_end() {
            parse_math_expression(tokens.slice(start..tokens.all().len()))?;
        }
        call.missing();
        return call.help_call();
    };
    let condition = parse_math_expression(tokens.slice(start..start + condition.len()))?;
    let (body, body_value) = match parse_block_or_value_argument(tokens.working_set, block, "block")
    {
        Ok(parsed) => parsed,
        Err(ErrMode::Cut(ParseFailure::Error(diagnostic))) => {
            return call.call_after(*diagnostic);
        }
        Err(error) => return Err(error),
    };
    let span = call.keyword.span.merge(block.span);
    call.finish(Expression::new(
        Expr::While(While {
            condition: Box::new(condition),
            body,
            body_value,
        }),
        span,
    ))
}

/// `loop { block }`.
pub fn parse_loop<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let mut call = KeywordCall::start(&mut tokens)?;
    let Some(block) = call.positional(&mut tokens, "block")? else {
        return call.help_call();
    };
    // An item that is no block nu's call reports and goes on past (`loop [1]`).
    let (body, body_value) =
        match parse_block_or_value_argument(tokens.working_set, &block, "block") {
            Ok(parsed) => parsed,
            Err(ErrMode::Cut(ParseFailure::Error(diagnostic))) => {
                return call.call_after(*diagnostic);
            }
            Err(error) => return Err(error),
        };
    call.end(&mut tokens)?;
    let span = call.keyword.span.merge(block.span);
    call.finish(Expression::new(Expr::Loop(Loop { body, body_value }), span))
}

/// `try { block } [catch handler] [finally handler]`, the handlers in either
/// order, at most two.
///
/// ```text
/// try     = "try" block [ handler [ handler ] ]
/// handler = ( "catch" | "finally" ) closure
/// ```
///
/// nu's `try` has two optional positionals that each take `catch` or
/// `finally`, so `try {} finally {} catch {}` and even two `catch`es parse.
pub fn parse_try<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    let mut call = KeywordCall::start(&mut tokens)?;
    let Some(block) = call.positional(&mut tokens, "block")? else {
        return call.help_call();
    };
    // An item that is no block nu's call reports and goes on past (`try [1]`).
    let (body, body_value) = match parse_block_or_value_argument(working_set, &block, "block") {
        Ok(parsed) => parsed,
        Err(ErrMode::Cut(ParseFailure::Error(diagnostic))) => return call.call_after(*diagnostic),
        Err(error) => return Err(error),
    };
    let mut span = call.keyword.span.merge(block.span);
    let mut handlers = Vec::new();
    loop {
        // `try {} --`: the marker may be the last item.
        if call.call_flags(&mut tokens)? {
            return call.help_call();
        }
        if tokens.at_end() {
            break;
        }
        // After two handlers nu's call has no positional left: a third is an extra positional
        // (ExtraPositional, the consumer's), and the statement nu's call.
        if handlers.len() == 2 {
            call.end(&mut tokens)?;
            break;
        }
        // nu's call reads a handler with its `oneof` of keyword shapes, which reports an item
        // that is neither keyword (ExpectedKeyword), a keyword without a value
        // (KeywordMissingArgument) or a value that is no closure, and goes on: the statement is
        // that call, the error recorded as recovered.
        let handler = (|tokens: &mut Tokens<'_, 'a>| {
            let handler_keyword = expected(
                "`catch` or `finally`",
                alt((keyword("catch"), keyword("finally"))),
            )
            .parse_next(tokens)?;
            let handler = tokens.expect_item("closure")?;
            let body = parse_try_handler(working_set, &handler)?;
            Ok((handler_keyword, handler, body))
        })(&mut tokens);
        let (handler_keyword, handler, body) = match handler {
            Ok(handler) => handler,
            Err(ErrMode::Cut(ParseFailure::Error(diagnostic))) => {
                return call.call_after(*diagnostic);
            }
            Err(error) => return Err(error),
        };
        let kind = match tokens.text(&handler_keyword) {
            "catch" => HandlerKind::Catch,
            _ => HandlerKind::Finally,
        };
        span = span.merge(handler.span);
        let body = Box::new(body);
        handlers.push(Handler {
            kind,
            keyword: handler_keyword.span,
            body,
        });
    }
    let keyword = call.keyword.span;
    call.finish(Expression::new(
        Expr::Try(Try {
            keyword,
            body,
            body_value,
            handlers,
        }),
        span,
    ))
}

/// A `catch`/`finally` handler: a closure, or a variable, subexpression or
/// brace value ([`parse_brace_value`]) that may hold one.
fn parse_try_handler<'a>(
    working_set: &WorkingSet<'a>,
    token: &Token,
) -> ParseResult<Expression<'a>> {
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
    if call.call_flags(&mut tokens)? {
        return call.help_call();
    }
    let value = match tokens.at_end() {
        true => None,
        false => {
            let value = tokens.expect_item("value")?;
            Some(Box::new(parse_value(
                tokens.working_set,
                value.span,
                ExpectedShape::Any,
            )?))
        }
    };
    call.end(&mut tokens)?;
    let span = value.as_ref().map_or(call.keyword.span, |value| {
        call.keyword.span.merge(value.span)
    });
    call.finish(Expression::new(Expr::Return(Return { value }), span))
}

/// `break` or `continue`, which take no arguments.
pub fn parse_break_or_continue<'a>(
    mut tokens: Tokens<'_, 'a>,
    expr: Expr<'a>,
) -> ParseResult<Expression<'a>> {
    let mut call = KeywordCall::start(&mut tokens)?;
    call.end(&mut tokens)?;
    let span = call.keyword.span;
    call.finish(Expression::new(expr, span))
}
