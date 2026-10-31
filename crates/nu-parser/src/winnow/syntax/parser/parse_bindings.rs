//! `let`, `mut` and `const` (nu-parser's `parse_bindings.rs`).

use winnow::error::ErrMode;

use crate::winnow::syntax::ast::{Binding, Expr, Expression};
use crate::winnow::syntax::error::Diagnostic;
use crate::winnow::syntax::input::{ParseFailure, ParseResult, cut};
use crate::winnow::syntax::lex::{AssignmentOperator, TokenContents};
use crate::winnow::syntax::span::Spanned;

use super::parse_keywords::{
    KeywordBoundary, call_after_error, keyword_boundary_with, keyword_flag, parse_help_call,
};
use super::parse_pipelines::parse_value_block;
use super::parse_signatures::{parse_type_after_var, parse_var_with_opt_type};
use super::tokens::Tokens;

/// Which of the three binding statements.
#[derive(Clone, Copy, PartialEq, Eq)]
enum BindingKind {
    Let,
    Mut,
    Const,
}

/// `let name[: type] [= value...]` (nu's `parse_let`).
pub fn parse_let<'a>(tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    parse_binding(tokens, BindingKind::Let)
}

/// `mut name[: type] [= value...]` (nu's `parse_mut`).
pub fn parse_mut<'a>(tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    parse_binding(tokens, BindingKind::Mut)
}

/// `const name[: type] [= value...]` (nu's `parse_const`).
pub fn parse_const<'a>(tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    parse_binding(tokens, BindingKind::Const)
}

/// `let`, `mut` and `const`: `keyword name[: type] [= value...]`. With an `=`
/// nu takes these arguments by position, so a `--` is not an end-of-options
/// marker (`let -- x = 1` is an error); without one it parses the statement as
/// an ordinary call, where `--` ends the options (`let -- x` declares `x`).
/// That call leaves the value out: optional for `let`, a missing positional of
/// `mut` and `const` that the consumer reports (`const x`). A typed `mut` or
/// `const` without a value stays an error: of their two required positionals
/// nu gives the variable only the `x:` item, which then has no type
/// (`MissingType`).
fn parse_binding<'a>(mut tokens: Tokens<'_, 'a>, kind: BindingKind) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    let statement = tokens;
    let (keyword_name, make_expr): (&str, fn(Box<Binding<'a>>) -> Expr<'a>) = match kind {
        BindingKind::Let => ("let", Expr::Let),
        BindingKind::Mut => ("mut", Expr::Mut),
        BindingKind::Const => ("const", Expr::Const),
    };
    // The whole statement, keyword included: `tokens.position()` and `equals_index` index it.
    let items = tokens.all();
    let keyword = tokens.expect_item(keyword_name)?;
    let has_equals = items
        .iter()
        .any(|token| matches!(token.contents, TokenContents::AssignmentOperator(_)));
    // nu reads the statement as its binding only with four items or more and a plain `=` after
    // the name, with a value after it (`parse_let`); any other statement is its call to the
    // keyword, whose first error is its own: a flag it does not have (`let --foo x`), a missing
    // name (`let = 1`, MissingPositional, the consumer's), or another word in the `=`'s place
    // (`let x += 1`, ExpectedKeyword) or nothing after it (`let x =`, KeywordMissingArgument),
    // which nu reports and goes on past. In a binding a flag is no name (`let --help x = 1`).
    let is_assign = |index: usize| {
        items.get(index).is_some_and(|token| {
            token.contents == TokenContents::AssignmentOperator(AssignmentOperator::Assign)
        })
    };
    let binds = items.len() >= 4 && (2..items.len() - 1).any(is_assign);
    let boundary = match binds {
        false => keyword_flag(&mut tokens, &[], false),
        true => keyword_boundary_with(&mut tokens, keyword_name, &[], false)?,
    };
    if let KeywordBoundary::Help | KeywordBoundary::UnknownFlag = boundary {
        if binds {
            let at = tokens
                .peek_token()
                .map_or(tokens.here(), |token| token.span);
            return Err(cut(Diagnostic::expected("valid variable name", at)));
        }
        return parse_help_call(statement);
    }
    if has_equals && !binds {
        if is_assign(1) {
            return parse_help_call(statement);
        }
        let operator = items
            .iter()
            .position(|token| matches!(token.contents, TokenContents::AssignmentOperator(_)))
            .map_or(keyword.span, |index| items[index].span);
        let error = match is_assign(items.len() - 1) {
            true => Diagnostic::expected("value after `=`", operator.past()),
            false => Diagnostic::expected("`=`", operator),
        };
        return call_after_error(statement, cut(error), parse_help_call);
    }
    if !has_equals
        && let Some(&marker) = tokens.peek_token()
        && working_set.get_span_contents(marker.span) == "--"
    {
        working_set.add_ignored(marker.span);
        tokens.next_token();
    }
    let name_token = match tokens.peek_token() {
        Some(token) if token.contents == TokenContents::Item => *token,
        _ => return Err(cut(Diagnostic::expected("variable name", tokens.here()))),
    };
    tokens.next_token();
    let name_text = working_set.get_span_contents(name_token.span);
    let (name, typed) = match parse_var_with_opt_type(working_set, &name_token) {
        Ok(parsed) => parsed,
        // nu reports an untyped name it refuses in a binding (`let $x-y = 1`, `let 'a b' = 1`,
        // `let -- x = 1`) and goes on with garbage for the variable, as the consumer does: the
        // error is recorded as recovered and the name kept as written.
        Err(ErrMode::Cut(ParseFailure::Error(diagnostic)))
            if binds && !name_text.ends_with(':') =>
        {
            working_set.error(diagnostic.as_recovered());
            (Spanned::new(name_text, name_token.span), false)
        }
        Err(error) => return Err(error),
    };
    let equals_index = items
        .iter()
        .position(|token| matches!(token.contents, TokenContents::AssignmentOperator(_)))
        .unwrap_or(items.len());
    let equals = items.get(equals_index).copied();
    if let Some(equals) = equals
        && equals.contents != TokenContents::AssignmentOperator(AssignmentOperator::Assign)
    {
        return Err(cut(Diagnostic::expected("`=`", equals.span)));
    }
    if equals.is_none() && !typed {
        // `let x --help`, `let x --foo`: nu's call, which reads an item after the name as its `=`
        // keyword, so another word there is its ExpectedKeyword, reported with `--help` too
        // (`const x --help f`), while an unknown flag comes first (UnknownFlag, semantic).
        match keyword_flag(&mut tokens, &[], true) {
            KeywordBoundary::Help => {
                let word = tokens.remaining().iter().find(|token| {
                    token.contents == TokenContents::Item && !tokens.text(token).starts_with('-')
                });
                return match word {
                    Some(word) => call_after_error(
                        statement,
                        cut(Diagnostic::expected("`=`", word.span)),
                        parse_help_call,
                    ),
                    None => parse_help_call(statement),
                };
            }
            KeywordBoundary::UnknownFlag => return parse_help_call(statement),
            KeywordBoundary::Argument => {}
        }
    }
    let ty = parse_type_after_var(
        working_set,
        &items[tokens.position()..equals_index],
        typed,
        name_token.span.past(),
        binds,
    )?;
    let (value, end) = match equals {
        // nu parses everything after the `=` as a block of its own (`let x = ls | length`).
        Some(equals) => {
            let rhs = tokens.slice(equals_index + 1..items.len());
            let Some(rhs_span) = rhs.span() else {
                return Err(cut(Diagnostic::expected(
                    "value after `=`",
                    equals.span.past(),
                )));
            };
            (Some(parse_value_block(rhs, rhs_span)?), rhs_span)
        }
        // nu's call gives a typed variable without a value only its name item, a missing type
        // (MissingType), and goes on: the statement is that call, the error recorded as
        // recovered.
        None if kind != BindingKind::Let && typed => {
            let diagnostic = Diagnostic::message(
                "missing required positional argument",
                items[items.len() - 1].span.past(),
            )
            .with_help(format!(
                "`{keyword_name}` needs a value: `{keyword_name} {} = <value>`",
                name.item
            ));
            return call_after_error(statement, cut(diagnostic), parse_help_call);
        }
        None => (None, items[items.len() - 1].span),
    };
    let binding = Binding {
        name,
        ty,
        eq: equals.map(|token| token.span),
        value,
    };
    Ok(Expression::new(
        make_expr(Box::new(binding)),
        keyword.span.merge(end),
    ))
}
