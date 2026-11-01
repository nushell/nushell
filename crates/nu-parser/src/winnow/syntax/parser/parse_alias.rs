//! `alias` (nu-parser's `parse_alias.rs`).

use std::borrow::Cow;

use winnow::error::ErrMode;

use crate::winnow::syntax::ast::{Alias, Call, Expr, Expression};
use crate::winnow::syntax::error::Diagnostic;
use crate::winnow::syntax::input::{ParseFailure, ParseResult, cut};
use crate::winnow::syntax::lex::{AssignmentOperator, Token, TokenContents, assignment_operator};
use crate::winnow::syntax::span::Spanned;

use super::WorkingSet;
use super::parse_calls::parse_call_lenient;
use super::parse_def::check_definition_name;
use super::parse_expressions::{
    ExpectedShape, is_math_expression_like, parse_builtin_commands, parse_math_expression,
    parse_value,
};
use super::parse_keywords::{
    ALIASABLE_PARSER_KEYWORDS, KeywordBoundary, UNALIASABLE_PARSER_KEYWORDS, call_after_error,
    keyword_boundary_with, parse_help_call,
};
use super::parse_signatures::parse_definition_name;
use super::tokens::Tokens;
use super::working_set::{AliasedKeyword, DeclKind};

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
    let Some((head, rest)) = words.split_first() else {
        return Ok(None);
    };
    // How many items after the keyword its signature requires.
    let complete = match working_set.get_span_contents(head.span) {
        "if" | "match" => 2,
        "try" => 1,
        _ => return Ok(None),
    };
    if rest
        .iter()
        .any(|token| working_set.get_span_contents(token.span).starts_with('-'))
    {
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

/// How a call to an alias of `call` parses: with external arguments when its
/// command takes them, as a call that changes which commands exist when that is
/// an overlay command, and, when the alias gives no arguments, as its command
/// when that is a row-condition command or `if`, `match` or `try` (`alias m =
/// match`, and an alias of such an alias).
fn call_alias_kind(working_set: &WorkingSet<'_>, call: &Call<'_>) -> DeclKind {
    let kind = working_set.find_decl(&call.head.name);
    if kind == Some(DeclKind::Wrapped) {
        return DeclKind::Wrapped;
    }
    // An alias of an overlay command (`alias ou = overlay use`, with arguments or not), or of
    // such an alias.
    if kind == Some(DeclKind::OverlayAlias)
        || matches!(
            call.head.name.as_ref(),
            "overlay use" | "overlay hide" | "overlay new"
        ) && !working_set.is_declared(&call.head.name)
    {
        return DeclKind::OverlayAlias;
    }
    if !call.arguments.is_empty() {
        return DeclKind::Declared;
    }
    match kind {
        Some(kind @ (DeclKind::RowCondition | DeclKind::KeywordAlias(_))) => kind,
        // A command of the file named `match` is no keyword.
        _ if working_set.is_declared(&call.head.name) => DeclKind::Declared,
        _ => AliasedKeyword::from_name(&call.head.name)
            .map_or(DeclKind::Declared, DeclKind::KeywordAlias),
    }
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

/// Whether `text` is a batch of short flags with an `h` in it (`-h`, `-xh`), which makes nu's
/// call parser see a help call whatever else the batch holds. A negative number (`-1`) or `-.5`
/// is no batch.
pub(super) fn short_flags_ask_for_help(text: &str) -> bool {
    text.strip_prefix('-').is_some_and(|batch| {
        !batch.starts_with(['-', '.'])
            && !batch.starts_with(|c: char| c.is_ascii_digit())
            && batch.contains('h')
    })
}

/// The statement as an ordinary call to `alias` or `type`, with `=` an
/// ordinary word, which is what nu keeps when it shows help (`alias --help =
/// ls`, `type t --help = int`).
pub(super) fn parse_alias_as_call<'a>(statement: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let words: Vec<Token> = statement
        .remaining()
        .iter()
        .map(|token| Token {
            contents: TokenContents::Item,
            span: token.span,
        })
        .collect();
    parse_help_call(Tokens::new(
        statement.working_set,
        &words,
        statement.end_span().start,
    ))
}

/// `alias name = target` (nu's `parse_alias`), also after `export`.
///
/// The checks below run in nu's order, which decides what a malformed alias
/// reports: the positions of the items first, then a help flag, then the
/// name, then the `=` and the target.
pub fn parse_alias<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    let statement = tokens;
    let keyword = tokens.expect_item("alias")?;
    // With fewer than three items after `alias` nu keeps the call it read, which declares
    // nothing (`alias ll ls`, `alias ll =`, `alias ll=ls`, `alias ll -x`, whose call's errors
    // nu drops): the consumer reports it (IncorrectValue, semantic).
    let incomplete = statement.remaining().len() < 4;
    // nu reports misplaced items and keeps garbage, which the consumer gives for the call.
    if let Err(error) = check_alias_name(working_set, tokens.remaining()) {
        return call_after_error(statement, error, parse_alias_as_call);
    }
    // nu drops the errors of the alias call and keeps it when it asks for
    // help, so a short-flag batch with an `h` before the `=` (`alias -hx`,
    // `alias x -xh`) is a help call whatever else the batch holds.
    let asks_for_help = tokens
        .remaining()
        .iter()
        .take_while(|token| token.contents == TokenContents::Item)
        .any(|token| short_flags_ask_for_help(working_set.get_span_contents(token.span)));
    if asks_for_help {
        return parse_alias_as_call(statement);
    }
    // Then nu parses the alias call and returns it when it is a help call:
    // `alias --help = ls` and `alias x --help` show help, and `--` is a name.
    // A flag nu's call does not know is no name either (`alias -x = ls`): it reads the item
    // after it as the name, as below.
    match keyword_boundary_with(&mut tokens, "alias", &[], false) {
        Ok(KeywordBoundary::Help) => return parse_alias_as_call(statement),
        Err(_) if incomplete => return parse_alias_as_call(statement),
        Err(error) => return call_after_error(statement, error, parse_alias_as_call),
        Ok(_) => {}
    };
    let name_token = tokens.expect_item("alias name")?;
    // A lone `-` is no flag for nu and so a name; `-x` and `--` are not, and
    // neither is a variable, subexpression or block, which is no string. nu
    // reports a name it refuses (`alias_name`: no string, an unsupported or
    // keyword name) and keeps the call to `alias` as garbage, which the consumer
    // does too: the error is recorded as recovered and no alias is declared.
    let name_text = working_set.get_span_contents(name_token.span);
    let not_a_name = Diagnostic::message("alias name not supported", name_token.span)
        .with_help("a bare alias name cannot start with `-`; quote it");
    // nu's call reads `--` as the end of its options and the item after it as the name
    // (`alias -- = ls` names `=`), which it refuses (AliasNotValid), with garbage.
    if name_text.starts_with('-') && name_text.len() > 1 {
        return call_after_error(statement, cut(not_a_name), parse_alias_as_call);
    }
    let not_a_string = name_text.starts_with(['$', '(', '{']);
    let name = match not_a_string {
        true => {
            working_set.error(not_a_name.as_recovered());
            Spanned::new(Cow::Borrowed(name_text), name_token.span)
        }
        false => parse_definition_name(working_set, name_token.span)?,
    };
    // `alias alias --help` is a help call before the name is checked.
    match keyword_boundary_with(&mut tokens, "alias", &[], false) {
        Ok(KeywordBoundary::Help) => return parse_alias_as_call(statement),
        Err(_) if incomplete => return parse_alias_as_call(statement),
        boundary => boundary?,
    };
    let declares = !not_a_string
        && match check_definition_name(&name, "alias") {
            Ok(()) => true,
            Err(ErrMode::Cut(ParseFailure::Error(diagnostic))) => {
                working_set.error(diagnostic.as_recovered());
                false
            }
            Err(error) => return Err(error),
        };
    let equals = match tokens.next_token() {
        Some(token)
            if token.contents == TokenContents::AssignmentOperator(AssignmentOperator::Assign) =>
        {
            *token
        }
        _ if incomplete => return parse_alias_as_call(statement),
        _ => return Err(cut(Diagnostic::expected("`=`", tokens.here()))),
    };
    // Nushell hands everything after `=` to the call parser as plain words,
    // so `alias ll = ls | length` is `ls` with the arguments `|` and `length`,
    // and `alias x = FOO=1 ls` calls the external command `FOO=1`.
    let words: Vec<Token> = tokens
        .remaining()
        .iter()
        .map(|token| Token {
            contents: TokenContents::Item,
            span: token.span,
        })
        .collect();
    let Some(first) = words.first() else {
        if incomplete {
            return parse_alias_as_call(statement);
        }
        return Err(cut(Diagnostic::expected(
            "command after `=`",
            equals.span.past(),
        )));
    };
    let first_text = working_set.get_span_contents(first.span);
    if !matches!(first_text, "if" | "match") && is_math_expression_like(first_text) {
        let diagnostic = Diagnostic::message("cannot create an alias to an expression", first.span)
            .with_help("an alias names a command and its arguments, such as `alias ll = ls -l`");
        // nu reports it, naming the expression its text reads as, and keeps the call to
        // `alias` without declaring one: so does the consumer, from the target's text, every
        // token after the `=` a part of it, a `|` or a redirection too (nu's lite parser takes
        // them all after an assignment operator).
        working_set.error(diagnostic.as_recovered());
        let last = words.last().unwrap_or(first);
        let value = Expression::new(Expr::Garbage, first.span.merge(last.span));
        let span = keyword.span.merge(value.span);
        return Ok(Expression::new(
            Expr::Alias(Alias {
                name,
                eq: equals.span,
                value: Box::new(value),
            }),
            span,
        ));
    }
    // Like nu, only the aliasable keywords (`if`, `match`, `try`, `overlay ...`)
    // may be aliased; `alias d = def` is an error. nu reads the target as a call
    // twice, the first time as the argument of `alias` (`parse_expression`, which
    // reads a keyword's call by the keyword's signature, its errors dropped), and
    // refuses to declare the alias (`finish_alias`), the statement kept, or makes
    // the target garbage when the second reading reports an error: so does the
    // consumer, the error recorded as recovered. The first reading of `run` and
    // `where`, and of an assignment, is no call, so those stay an error.
    let target: String = words
        .iter()
        .take(2)
        .map(|token| working_set.get_span_contents(token.span))
        .collect::<Vec<_>>()
        .join(" ");
    let single = first_text;
    let keyword_words =
        match words.len() > 1 && UNALIASABLE_PARSER_KEYWORDS.contains(&target.as_str()) {
            true => Some(2),
            false => UNALIASABLE_PARSER_KEYWORDS.contains(&single).then_some(1),
        };
    if let Some(keyword_words) = keyword_words {
        let diagnostic =
            Diagnostic::message("cannot create an alias to a parser keyword", first.span)
                .with_help(format!(
                    "only {} can be aliased",
                    ALIASABLE_PARSER_KEYWORDS.join(", ")
                ));
        let read_otherwise = matches!(first_text, "run" | "where")
            || words.iter().any(|word| {
                assignment_operator(working_set.get_span_contents(word.span)).is_some()
            });
        if words.len() != keyword_words && read_otherwise {
            return Err(cut(diagnostic));
        }
        working_set.error(diagnostic.as_recovered());
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
        Expr::Call(call) => call_alias_kind(working_set, call),
        _ => DeclKind::Declared,
    };
    if declares && keyword_words.is_none() {
        working_set.add_alias(&name.item, kind);
    }
    let span = keyword.span.merge(value.span);
    Ok(Expression::new(
        Expr::Alias(Alias {
            name,
            eq: equals.span,
            value: Box::new(value),
        }),
        span,
    ))
}
