//! `alias` (nu-parser's `parse_alias.rs`).

use crate::ast::{Alias, Expr, Expression};
use crate::error::Diagnostic;
use crate::input::{ParseResult, cut};
use crate::lex::{AssignmentOperator, Token, TokenContents};

use super::WorkingSet;
use super::parse_calls::parse_call_lenient;
use super::parse_def::check_definition_name;
use super::parse_expressions::{
    ExpectedShape, is_math_expression_like, parse_builtin_commands, parse_math_expression, parse_value,
};
use super::parse_keywords::{
    ALIASABLE_PARSER_KEYWORDS, KeywordBoundary, UNALIASABLE_PARSER_KEYWORDS, keyword_boundary_with, parse_help_call,
};
use super::parse_signatures::parse_definition_name;
use super::tokens::Tokens;
use super::working_set::DeclKind;

/// An aliased `if`, `match` or `try`, which nu parses with the keyword's own
/// signature, forgiving only missing positionals: a complete one is parsed as
/// the statement (`alias i = if x y` has no block, `alias t = try {} catch`
/// no handler), and of an incomplete one the items there are parsed
/// (`alias i = if x` is fine, `alias i = if not` fails in its condition).
/// `None` for anything else, and for targets with flags, which the call
/// parser checks.
fn parse_aliased_keyword<'a>(
    working_set: &WorkingSet<'a>,
    words: &[Token],
    end: usize,
) -> ParseResult<Option<Expression<'a>>> {
    let Some((head, rest)) = words.split_first() else { return Ok(None) };
    // How many items after the keyword its signature requires.
    let complete = match working_set.get_span_contents(head.span) {
        "if" | "match" => 2,
        "try" => 1,
        _ => return Ok(None),
    };
    if rest.iter().any(|token| working_set.get_span_contents(token.span).starts_with('-')) {
        return Ok(None);
    }
    if rest.len() >= complete {
        return parse_builtin_commands(Tokens::new(working_set, words, end)).map(Some);
    }
    match (working_set.get_span_contents(head.span), rest) {
        ("if", [_]) => {
            parse_math_expression(Tokens::new(working_set, rest, end))?;
        }
        ("match", [value]) => {
            parse_value(working_set, value.span, ExpectedShape::Any)?;
        }
        _ => {}
    }
    Ok(None)
}

/// nu's `check_alias_name`, which looks at the items after `alias` by
/// position before the call is parsed: with three or more, the second must be
/// `=` (`alias --help x = ls` and `alias x --help extra` are "missing sign");
/// with one or two, the first must not be (`alias = ls` is "missing name").
fn check_alias_name(working_set: &WorkingSet<'_>, items: &[Token]) -> ParseResult<()> {
    let text = |index: usize| working_set.get_span_contents(items[index].span);
    match items.len() {
        0 => Ok(()),
        1 | 2 if text(0) == "=" => Err(cut(Diagnostic::expected("alias name", items[0].span))),
        1 | 2 => Ok(()),
        _ if text(1) != "=" => Err(cut(Diagnostic::expected("`=`", items[1].span))),
        _ => Ok(()),
    }
}

/// The statement as an ordinary call to `alias`, with `=` an ordinary word,
/// which is what nu keeps when it shows help (`alias --help = ls`) or when an
/// exported alias has one item and no `=` after its name (`export alias h home`).
fn parse_alias_as_call<'a>(statement: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let words: Vec<Token> =
        statement.remaining().iter().map(|token| Token { contents: TokenContents::Item, span: token.span }).collect();
    parse_help_call(Tokens::new(statement.working_set, &words, statement.end_span().start))
}

/// `alias name = target` (nu's `parse_alias`); `exported` for `export alias`,
/// which nu lets go without a target (`export alias x =`) because its length
/// check counts the `export` word.
///
/// The checks below run in nu's order, which decides what a malformed alias
/// reports: the positions of the items first, then a help flag, then the
/// name, then the `=` and the target.
pub fn parse_alias<'a>(mut tokens: Tokens<'_, 'a>, exported: bool) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    let statement = tokens;
    let keyword = tokens.expect_item("alias")?;
    check_alias_name(working_set, tokens.remaining())?;
    // nu drops the errors of the alias call and keeps it when it asks for
    // help, so a short-flag batch with an `h` before the `=` (`alias -hx`,
    // `alias x -xh`) is a help call whatever else the batch holds.
    let asks_for_help = tokens
        .remaining()
        .iter()
        .take_while(|token| token.contents == TokenContents::Item)
        .map(|token| working_set.get_span_contents(token.span))
        .any(|text| {
            text.strip_prefix('-').is_some_and(|batch| {
                !batch.starts_with(['-', '.'])
                    && !batch.starts_with(|c: char| c.is_ascii_digit())
                    && batch.contains('h')
            })
        });
    if asks_for_help {
        return parse_alias_as_call(statement);
    }
    // Then nu parses the alias call and returns it when it is a help call:
    // `alias --help = ls` and `alias x --help` show help, and `--` is a name.
    if let KeywordBoundary::Help = keyword_boundary_with(&mut tokens, "alias", &[], false)? {
        return parse_alias_as_call(statement);
    }
    let name_token = tokens.expect_item("alias name")?;
    // A lone `-` is no flag for nu and so a name; `-x` and `--` are not, and
    // neither is a variable, subexpression or block, which is no string.
    let name_text = working_set.get_span_contents(name_token.span);
    if (name_text.starts_with('-') && name_text.len() > 1) || name_text.starts_with(['$', '(', '{']) {
        return Err(cut(Diagnostic::message("alias name not supported", name_token.span)
            .with_help("a bare alias name cannot start with `-`; quote it")));
    }
    let name = parse_definition_name(working_set, name_token.span)?;
    // `alias alias --help` is a help call before the name is checked.
    if let KeywordBoundary::Help = keyword_boundary_with(&mut tokens, "alias", &[], false)? {
        return parse_alias_as_call(statement);
    }
    check_definition_name(&name, "alias")?;
    // nu creates an alias only from three items after `alias`; with `export`
    // counted, `export alias h home` is four items, has no `=` to miss, and
    // defines nothing.
    if exported
        && let [only] = tokens.remaining()
        && only.contents != TokenContents::AssignmentOperator(AssignmentOperator::Assign)
    {
        return parse_alias_as_call(statement);
    }
    let equals = match tokens.next_token() {
        Some(token) if token.contents == TokenContents::AssignmentOperator(AssignmentOperator::Assign) => *token,
        _ => return Err(cut(Diagnostic::expected("`=`", tokens.here()))),
    };
    // Nushell hands everything after `=` to the call parser as plain words,
    // so `alias ll = ls | length` is `ls` with the arguments `|` and `length`,
    // and `alias x = FOO=1 ls` calls the external command `FOO=1`.
    let words: Vec<Token> =
        tokens.remaining().iter().map(|token| Token { contents: TokenContents::Item, span: token.span }).collect();
    let Some(first) = words.first() else {
        if exported {
            let alias = Alias { name, eq: equals.span, value: None };
            return Ok(Expression::new(Expr::Alias(alias), keyword.span.merge(equals.span)));
        }
        return Err(cut(Diagnostic::expected("command after `=`", equals.span.past())));
    };
    let first_text = working_set.get_span_contents(first.span);
    if !matches!(first_text, "if" | "match") && is_math_expression_like(first_text) {
        return Err(cut(Diagnostic::message("cannot create an alias to an expression", first.span)
            .with_help("an alias names a command and its arguments, such as `alias ll = ls -l`")));
    }
    // Like nu, only the aliasable keywords (`if`, `match`, `try`, `overlay ...`)
    // may be aliased; `alias d = def` is an error.
    let target: String =
        words.iter().take(2).map(|token| working_set.get_span_contents(token.span)).collect::<Vec<_>>().join(" ");
    let single = first_text;
    if UNALIASABLE_PARSER_KEYWORDS.contains(&target.as_str()) || UNALIASABLE_PARSER_KEYWORDS.contains(&single) {
        return Err(cut(Diagnostic::message("cannot create an alias to a parser keyword", first.span)
            .with_help(format!("only {} can be aliased", ALIASABLE_PARSER_KEYWORDS.join(", ")))));
    }
    // nu forgives missing positionals and flag values in an alias target
    // (`alias x = overlay new`), but not unknown flags.
    let end = tokens.end_span().start;
    let value = match parse_aliased_keyword(working_set, &words, end)? {
        Some(value) => value,
        None => parse_call_lenient(Tokens::new(working_set, &words, end), true)?,
    };
    // nu declares the alias now, not before the block (`g x; alias g = ls`
    // calls an unknown `g`); a call to an alias of an external command is an
    // external call, and one to an alias of a wrapped command takes external
    // arguments.
    let kind = match &value.expr {
        Expr::ExternalCall(_) => DeclKind::ExternalAlias,
        Expr::Call(call) if working_set.find_decl(&call.head.name) == Some(DeclKind::Wrapped) => DeclKind::Wrapped,
        _ => DeclKind::Declared,
    };
    working_set.add_alias(&name.item, kind);
    let span = keyword.span.merge(value.span);
    Ok(Expression::new(Expr::Alias(Alias { name, eq: equals.span, value: Some(Box::new(value)) }), span))
}
