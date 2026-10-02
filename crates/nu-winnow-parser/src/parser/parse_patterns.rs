//! `match` patterns (nu-parser's `parse_patterns.rs`).
//!
//! ```text
//! pattern        = "_" | "$" name | list-pattern | record-pattern | value
//! list-pattern   = "[" { pattern } [ ".." | "..$" name ] "]"
//! record-pattern = "{" { key ":" pattern | "$" name } "}"
//! ```

use std::borrow::Cow;

use winnow::Parser;
use winnow::combinator::{eof, not, opt, preceded, repeat, repeat_till};

use crate::ast::{MatchPattern, Pattern};
use crate::error::Diagnostic;
use crate::input::{ParseResult, cut};
use crate::lex::{LexOptions, Token, TokenContents, lex};
use crate::span::{Span, Spanned};

use super::WorkingSet;
use super::lite_parser::lite_parse_parts;
use super::parse_expressions::{ExpectedShape, parse_value};
use super::parse_helpers::{declared_variable_name, delimited_interior, is_variable};
use super::parse_signatures::ensure_not_reserved_variable_name;
use super::tokens::{Tokens, cut_with, expected, item, keyword};

/// Parse the pattern item at `span`.
pub fn parse_pattern<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<MatchPattern<'a>> {
    let text = working_set.get_span_contents(span);
    let pattern = match text.as_bytes()[0] {
        b'$' => Pattern::Variable(parse_variable_pattern(working_set, span)?),
        b'{' => Pattern::Record(parse_record_pattern(working_set, span)?),
        b'[' => Pattern::List(parse_list_pattern(working_set, span)?),
        b'_' if text == "_" => Pattern::IgnoreValue,
        _ => Pattern::Expression(Box::new(parse_value(working_set, span, ExpectedShape::Any)?)),
    };
    Ok(MatchPattern { span, pattern })
}

/// The next item, parsed as a pattern.
pub fn pattern<'a>(tokens: &mut Tokens<'_, 'a>) -> ParseResult<MatchPattern<'a>> {
    let token = item(tokens)?;
    parse_pattern(tokens.working_set, token.span)
}

/// The name of a `$var` pattern, which binds a variable and so may not be a
/// reserved name.
fn parse_variable_pattern<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<&'a str> {
    let text = working_set.get_span_contents(span);
    if !is_variable(text) {
        return Err(cut(Diagnostic::expected("valid variable name", span)));
    }
    let (_, name) = declared_variable_name(text);
    ensure_not_reserved_variable_name(name, span)?;
    Ok(name)
}

/// The tokens of a pattern's interior, comments recorded and dropped, every
/// token taken as an item (`[a = b]` has three).
fn pattern_interior(working_set: &WorkingSet<'_>, inner: Span, options: LexOptions) -> ParseResult<Vec<Token>> {
    let lexed = lex(working_set.get_span_contents(inner), inner.start, options).map_err(cut)?;
    working_set.add_comments(&lexed);
    Ok(lexed
        .into_iter()
        .filter(|token| !matches!(token.contents, TokenContents::Eof | TokenContents::Comment))
        .collect())
}

/// `[p1, p2, ..$rest]`. Like nu, the interior is lite-parsed: `|` separates
/// groups (`[1 | 2]` is `[1, 2]`), a redirection is dropped, and the items
/// after a `..`/`..$rest` in the same group are dropped too.
fn parse_list_pattern<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<Vec<MatchPattern<'a>>> {
    let inner = delimited_interior(working_set, span, "[", "]")?;
    let lexed = pattern_interior(working_set, inner, LexOptions::PATTERN_LIST)?;
    if let Some(semicolon) = lexed.iter().find(|token| token.contents == TokenContents::Semicolon) {
        return Err(cut(Diagnostic::message("unexpected semicolon in list pattern", semicolon.span)
            .with_help("use commas or whitespace to separate list items")));
    }
    let mut patterns = Vec::new();
    for group in lite_parse_parts(working_set, &lexed)? {
        let group: Vec<Token> =
            group.iter().map(|token| Token { contents: TokenContents::Item, span: token.span }).collect();
        let mut tokens = Tokens::new(working_set, &group, inner.end);
        let items: Vec<MatchPattern<'a>> = repeat(0.., preceded(not(rest_marker), pattern)).parse_next(&mut tokens)?;
        patterns.extend(items);
        if let Some(rest) = opt(rest_pattern).parse_next(&mut tokens)? {
            patterns.push(rest);
            // nu stops reading the group here.
            for ignored in tokens.remaining() {
                working_set.add_ignored(ignored.span);
            }
        }
    }
    Ok(patterns)
}

/// `..` or `..$rest` (but not the `...` of a spread, nor a range like `..5`).
fn is_rest_marker(text: &str) -> bool {
    text.strip_prefix("..").is_some_and(|name| name.is_empty() || name.starts_with('$'))
}

/// The item of a `..` or `..$rest` pattern.
fn rest_marker(tokens: &mut Tokens<'_, '_>) -> ParseResult<Token> {
    let working_set = tokens.working_set;
    item.verify(|token| is_rest_marker(working_set.get_span_contents(token.span))).parse_next(tokens)
}

/// `..` (ignore the remaining items) or `..$rest` (bind them).
fn rest_pattern<'a>(tokens: &mut Tokens<'_, 'a>) -> ParseResult<MatchPattern<'a>> {
    let marker = rest_marker(tokens)?;
    let pattern = match marker.span.len() {
        2 => Pattern::IgnoreRest,
        _ => {
            let name_span = Span::new(marker.span.start + 2, marker.span.end);
            Pattern::Rest(Spanned::new(parse_variable_pattern(tokens.working_set, name_span)?, name_span))
        }
    };
    Ok(MatchPattern { span: marker.span, pattern })
}

/// `{key: pattern, $shorthand}`. Like nu, every token is a field name, kept
/// verbatim (`{"a": $x}` has the field `"a"`, quotes included), and must be
/// followed by `:` and a pattern.
fn parse_record_pattern<'a>(
    working_set: &WorkingSet<'a>,
    span: Span,
) -> ParseResult<Vec<(Spanned<Cow<'a, str>>, MatchPattern<'a>)>> {
    let inner = delimited_interior(working_set, span, "{", "}")?;
    let lexed: Vec<Token> = pattern_interior(working_set, inner, LexOptions::PATTERN_RECORD)?
        .into_iter()
        .map(|token| Token { contents: TokenContents::Item, span: token.span })
        .collect();
    let field = |tokens: &mut Tokens<'_, 'a>| {
        let key = item(tokens)?;
        if tokens.text(&key).starts_with('$') {
            // `{$name}` binds the field of the same name.
            let name = parse_variable_pattern(working_set, key.span)?;
            let pattern = MatchPattern { span: key.span, pattern: Pattern::Variable(name) };
            return Ok((Spanned::new(Cow::Borrowed(name), key.span), pattern));
        }
        cut_with(keyword(":"), |_| {
            Diagnostic::expected("record", span)
                .with_help("record patterns look like `{key: pattern}`; a `:` must follow the field name")
        })
        .parse_next(tokens)?;
        let pattern = expected("pattern for record field", pattern).parse_next(tokens)?;
        Ok((Spanned::new(Cow::Borrowed(tokens.text(&key)), key.span), pattern))
    };
    let mut tokens = Tokens::new(working_set, &lexed, inner.end);
    repeat_till(0.., field, eof).map(|(fields, _)| fields).parse_next(&mut tokens)
}
