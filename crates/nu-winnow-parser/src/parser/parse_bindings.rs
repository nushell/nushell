//! `let`, `mut` and `const` (nu-parser's `parse_bindings.rs`).

use crate::ast::{Binding, Expr, Expression};
use crate::error::Diagnostic;
use crate::input::{ParseResult, cut};
use crate::lex::{AssignmentOperator, TokenContents};

use super::parse_keywords::{KeywordBoundary, keyword_boundary, keyword_boundary_with, parse_help_call};
use super::parse_pipelines::parse_block;
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

/// `mut name[: type] = value...` (nu's `parse_mut`).
pub fn parse_mut<'a>(tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    parse_binding(tokens, BindingKind::Mut)
}

/// `const name[: type] = value...` (nu's `parse_const`).
pub fn parse_const<'a>(tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    parse_binding(tokens, BindingKind::Const)
}

/// `let`, `mut` and `const`: `keyword name[: type] [= value...]`. Only `let`
/// may leave the value out. With an `=` nu takes these arguments by position,
/// so a `--` is not an end-of-options marker (`let -- x = 1` is an error);
/// without one it parses `let` as an ordinary call, where `--` ends the
/// options (`let -- x` declares `x`).
fn parse_binding<'a>(mut tokens: Tokens<'_, 'a>, kind: BindingKind) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    let statement = tokens;
    let (keyword_name, make_expr): (&str, fn(Box<Binding<'a>>) -> Expr<'a>) = match kind {
        BindingKind::Let => ("let", Expr::Let),
        BindingKind::Mut => ("mut", Expr::Mut),
        BindingKind::Const => ("const", Expr::Const),
    };
    let items = tokens.all();
    let keyword = tokens.expect_item(keyword_name)?;
    if let KeywordBoundary::Help = keyword_boundary_with(&mut tokens, keyword_name, &[], false)? {
        return parse_help_call(statement);
    }
    let has_equals = items.iter().any(|token| matches!(token.contents, TokenContents::AssignmentOperator(_)));
    if matches!(kind, BindingKind::Let)
        && !has_equals
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
    let (name, typed) = parse_var_with_opt_type(working_set, &name_token)?;
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
        // `let x --help`
        if let KeywordBoundary::Help = keyword_boundary(&mut tokens, keyword_name, &[])? {
            return parse_help_call(statement);
        }
    }
    let ty = parse_type_after_var(working_set, &items[tokens.position()..equals_index], typed, name_token.span.past())?;
    let (value, end) = match equals {
        Some(equals) => {
            let rhs = tokens.slice(equals_index + 1..items.len());
            let Some(rhs_span) = rhs.span() else {
                return Err(cut(Diagnostic::expected("value after `=`", equals.span.past())));
            };
            (Some(parse_block(rhs, rhs_span)), rhs_span)
        }
        None if kind != BindingKind::Let => {
            return Err(cut(Diagnostic::message(
                "missing required positional argument",
                items[items.len() - 1].span.past(),
            )
            .with_help(format!("`{keyword_name}` needs a value: `{keyword_name} {} = <value>`", name.item))));
        }
        None => (None, items[items.len() - 1].span),
    };
    let binding = Binding { name, ty, eq: equals.map(|token| token.span), value };
    Ok(Expression::new(make_expr(Box::new(binding)), keyword.span.merge(end)))
}
