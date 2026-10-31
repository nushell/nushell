//! `type` (nu-parser's `parse_type_alias.rs`).

use crate::winnow::syntax::ast::{Expr, Expression, SyntaxShape, TypeAlias, TypeAnnotation};
use crate::winnow::syntax::error::{Diagnostic, ErrorKind};
use crate::winnow::syntax::input::{ParseResult, cut};
use crate::winnow::syntax::lex::{LexOptions, TokenContents};
use crate::winnow::syntax::span::{Span, Spanned};

use super::parse_alias::{parse_alias_as_call, short_flags_ask_for_help};
use super::parse_helpers::is_help_text;
use super::parse_keywords::call_after_error;
use super::parse_shape_specs::{BUILTIN_TYPE_NAMES, is_valid_type_alias_name, parse_type};
use super::tokens::Tokens;

/// `type name = <type>` (nu's `parse_type_alias`); `exported` for `export type`. The type is any
/// type a parameter annotation accepts, without a `@completer`, which belongs to a parameter.
///
/// The checks run in nu's order, which decides what a malformed statement reports: a help flag
/// among the first two items after the keyword, then the positions of the items, then the name,
/// then the type. The type's items are joined and read again as one type, as for a `let`
/// annotation (`type R = record<a: int, b: string>` is several items).
///
/// ```text
/// type-alias = [ "export" ] "type" name "=" type
/// ```
pub fn parse_type_alias<'a>(
    mut tokens: Tokens<'_, 'a>,
    exported: bool,
) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    let statement = tokens;
    let keyword = tokens.expect_item("type")?;
    let keyword_text = if exported { "export type" } else { "type" };
    let rest = tokens.remaining();
    let text = |index: usize| working_set.get_span_contents(rest[index].span);
    // nu parses the name and `=` as a call to `type` only to see whether it asks for help.
    let asks_for_help =
        |index: usize| is_help_text(text(index)) || short_flags_ask_for_help(text(index));
    if (0..rest.len().min(2))
        .any(|index| rest[index].contents == TokenContents::Item && asks_for_help(index))
    {
        return parse_alias_as_call(statement);
    }
    let missing_equals = |index: usize| {
        let name = text(index);
        let lookup = match exported {
            false => format!(
                " To see what the command `{name}` is, use `which {name}`; to run an external `type` program, use `^type {name}`."
            ),
            true => String::new(),
        };
        cut(
            Diagnostic::message("missing `=` in type alias", rest[index].span).with_help(format!(
                "`{keyword_text}` defines a type alias: `{keyword_text} {name} = <type>`.{lookup}"
            )),
        )
    };
    // nu reports items out of place (`type_alias_layout_error`) with garbage over the
    // statement, which the consumer reads from nu's call to `type`: the error is recorded as
    // recovered and the statement is that call.
    let layout_error = match rest.len() {
        0 => Some(cut(Diagnostic::expected(
            "type alias name",
            keyword.span.past(),
        )
        .with_help(format!("{keyword_text} <name> = <type>")))),
        _ if text(0) == "=" => Some(cut(Diagnostic::expected("type alias name", rest[0].span)
            .with_help("the name comes before `=`"))),
        1 => Some(match text(0).split_once('=') {
            Some((name, ty)) => cut(Diagnostic::message("`=` must stand alone", rest[0].span)
                .with_help(format!(
                    "put spaces around `=`: `{keyword_text} {name} = {ty}`"
                ))),
            None => missing_equals(0),
        }),
        _ if text(1) != "=" => Some(missing_equals(0)),
        _ => None,
    };
    if let Some(error) = layout_error {
        return call_after_error(statement, error, parse_alias_as_call);
    }
    let name = Spanned::new(text(0), rest[0].span);
    let eq = rest[1].span;
    // The type's items, or a point after the `=` when there are none.
    let type_span = match (rest.get(2), rest.last()) {
        (Some(first), Some(last)) => first.span.merge(last.span),
        _ => eq.past(),
    };
    // nu checks the name, then reads the type (`finish_type_alias`): it reports a built-in or
    // invalid name with garbage, and a missing type or one of several items with the alias
    // defined as `any`. Either way the error is recorded as recovered and the alias's type is
    // `any` over the type's items, which the consumer reads again.
    let recovered = |diagnostic: Diagnostic| {
        working_set.error(diagnostic.as_recovered());
        TypeAnnotation {
            span: type_span,
            shape: SyntaxShape::Any,
        }
    };
    let ty = if BUILTIN_TYPE_NAMES.contains(&name.item) {
        recovered(
            Diagnostic::message(format!("`{}` is a built-in type", name.item), name.span)
                .with_help("choose a name that isn't a built-in type"),
        )
    } else if !is_valid_type_alias_name(name.item) {
        recovered(Diagnostic::message("type alias name not valid", name.span).with_help(
            "a type alias name starts with a letter or `_` and continues with letters, digits, `_` and `-`",
        ))
    } else if type_span.is_empty() {
        recovered(Diagnostic::expected("type after `=`", type_span))
    } else {
        let lexed = working_set
            .lex(type_span, LexOptions::VAR_TYPE)
            .map_err(cut)?;
        match lexed
            .iter()
            .filter(|token| token.contents != TokenContents::Eof)
            .collect::<Vec<_>>()
            .as_slice()
        {
            [only] => parse_type(working_set, only.span)?,
            [_, extra, ..] => recovered(
                Diagnostic::new(
                    ErrorKind::ExtraTokens,
                    Span::new(extra.span.start, type_span.end),
                )
                .with_help("a type alias names one type"),
            ),
            [] => recovered(Diagnostic::expected("type after `=`", type_span)),
        }
    };
    let span = keyword.span.merge(type_span);
    Ok(Expression::new(
        Expr::TypeAlias(TypeAlias { name, eq, ty }),
        span,
    ))
}
