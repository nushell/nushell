//! Type annotations: shape names, generic shapes such as `list<int>` and
//! `record<a: int>`, and parameter completers (nu-parser's
//! `parse_shape_specs.rs`).

use winnow::Parser;
use winnow::combinator::{alt, opt};
use winnow::token::any;

use crate::ast::{Expr, SyntaxShape, TypeAnnotation, TypeField};
use crate::error::{Diagnostic, ErrorKind};
use crate::input::{ParseResult, cut};
use crate::lex::{LexOptions, TokenContents};
use crate::span::{Span, Spanned};

use super::WorkingSet;
use super::parse_expressions::{ExpectedShape, parse_value};
use super::tokens::{Tokens, cut_with, item, keyword, repeat_to_end};

/// A parameter's type item, which may carry a `@completer` suffix (nu's
/// `parse_shape_name`). Like nu, the
/// split is at the first `@` wherever it is (`record<a@b: int>` is then an
/// unclosed `record<`), and an empty type before the `@` is unknown.
pub fn parse_shape_name<'a>(
    working_set: &WorkingSet<'a>,
    span: Span,
) -> ParseResult<(TypeAnnotation<'a>, Option<Spanned<&'a str>>)> {
    let text = working_set.get_span_contents(span);
    let (type_text, completer) = match text.find('@') {
        Some(at) => (&text[..at], Some(Spanned::new(&text[at + 1..], Span::new(span.start + at + 1, span.end)))),
        None => (text, None),
    };
    let annotation = parse_type(working_set, Span::new(span.start, span.start + type_text.len()))?;
    if let Some(completer) = completer {
        parse_completer(working_set, completer)?;
    }
    Ok((annotation, completer))
}

/// A parameter completer (nu's `parse_completer`): the name of a command
/// (bare or quoted) or a list of
/// values; a subexpression or a record cannot be one. Whether the command
/// exists is the consumer's business (it may come from a `use`d module).
fn parse_completer(working_set: &WorkingSet<'_>, completer: Spanned<&str>) -> ParseResult<()> {
    let text = completer.item;
    let not_a_name = || {
        cut(Diagnostic::message(
            "the parameter completer must be a string (the name of a command) or a list",
            completer.span,
        ))
    };
    match text.as_bytes().first() {
        None => Err(cut(Diagnostic::expected("completer after `@`", completer.span))),
        Some(b'[') => parse_value(working_set, completer.span, ExpectedShape::Any).map(|_| ()),
        Some(b'$') => Ok(()),
        Some(b'(' | b'{') => Err(not_a_name()),
        Some(_) => match parse_value(working_set, completer.span, ExpectedShape::String)?.expr {
            Expr::String(_) => Ok(()),
            _ => Err(not_a_name()),
        },
    }
}

/// A type annotation such as `int`, `list<string>` or `record<a: int>` (nu's
/// `parse_type`, with `parse_shape_name`'s table of names).
pub fn parse_type<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<TypeAnnotation<'a>> {
    let text = working_set.get_span_contents(span);
    let shape = match text {
        "any" => SyntaxShape::Any,
        "binary" => SyntaxShape::Binary,
        "bool" => SyntaxShape::Boolean,
        "cell-path" => SyntaxShape::CellPath,
        "closure" => SyntaxShape::Closure,
        "datetime" => SyntaxShape::DateTime,
        "directory" => SyntaxShape::Directory,
        "duration" => SyntaxShape::Duration,
        "error" => SyntaxShape::Error,
        "external_arg" => SyntaxShape::ExternalArgument,
        "float" => SyntaxShape::Float,
        "filesize" => SyntaxShape::Filesize,
        "glob" => SyntaxShape::GlobPattern,
        "int" => SyntaxShape::Int,
        "nothing" => SyntaxShape::Nothing,
        "number" => SyntaxShape::Number,
        "path" => SyntaxShape::Filepath,
        "range" => SyntaxShape::Range,
        "string" => SyntaxShape::String,
        "block" => {
            return Err(cut(Diagnostic::message("blocks are not supported as first-class values", span)
                .with_help("use `closure` instead of `block`")));
        }
        _ if text.starts_with("list")
            || text.starts_with("record")
            || text.starts_with("table")
            || text.starts_with("oneof") =>
        {
            parse_generic_shape(working_set, span)?
        }
        _ => {
            let mut diagnostic = Diagnostic::new(ErrorKind::UnknownType(text.to_string()), span);
            if text.contains('@') {
                diagnostic = diagnostic.with_help("type specifications do not support custom completers here");
            }
            return Err(cut(diagnostic));
        }
    };
    Ok(TypeAnnotation { span, shape })
}

/// `list`, `record`, `table` or `oneof`, with or without `<...>` parameters
/// (nu's `parse_generic_shape`).
fn parse_generic_shape<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<SyntaxShape<'a>> {
    let text = working_set.get_span_contents(span);
    let (name, params) = match text.find('<') {
        None => (text, None),
        Some(open) => {
            let Some(inner) = text[open + 1..].strip_suffix('>') else {
                return Err(cut(Diagnostic::new(
                    ErrorKind::Unclosed { delimiter: ">", open: Span::new(span.start + open, span.start + open + 1) },
                    span.past(),
                )));
            };
            let inner_span = Span::new(span.start + open + 1, span.start + open + 1 + inner.len());
            (&text[..open], Some(inner_span))
        }
    };
    Ok(match name {
        "list" => {
            let element = match params {
                None => None,
                Some(params) => {
                    let types = parse_type_params(working_set, params)?;
                    if types.len() > 1 {
                        return Err(cut(Diagnostic::message("expected a single type parameter", params)));
                    }
                    types.into_iter().next().map(Box::new)
                }
            };
            SyntaxShape::List(element)
        }
        "oneof" => SyntaxShape::OneOf(match params {
            None => Vec::new(),
            Some(params) => parse_type_params(working_set, params)?,
        }),
        "record" => SyntaxShape::Record(match params {
            None => Vec::new(),
            Some(params) => parse_named_type_params(working_set, params)?,
        }),
        "table" => SyntaxShape::Table(match params {
            None => Vec::new(),
            Some(params) => parse_named_type_params(working_set, params)?,
        }),
        _ => return Err(cut(Diagnostic::new(ErrorKind::UnknownType(text.to_string()), span))),
    })
}

/// The comma-separated types of `list<int>` and `oneof<int, string>` (nu's
/// `parse_type_params`). Like nu, every other token is read as a type, so
/// `list<int;>` and `list<int:>` have an unknown type.
fn parse_type_params<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<Vec<TypeAnnotation<'a>>> {
    let tokens = working_set.lex(span, LexOptions::TYPE_PARAMS).map_err(cut)?;
    tokens
        .iter()
        .filter(|token| token.contents != TokenContents::Eof)
        .filter(|token| !working_set.get_span_contents(token.span).starts_with(','))
        .map(|token| parse_type(working_set, token.span))
        .collect()
}

/// The fields of `record<a: int, b>` or `table<...>` (nu's
/// `parse_named_type_params`):
///
/// ```text
/// fields = { "," | name [ ":" type | "," ] }
/// ```
///
/// Like nu, every token must be an item (a `;` or `|` is not a field name),
/// the type after a `:` may be any token (`record<a:, b: int>` has the unknown
/// type `,`), and a name without a type has the type `any`.
fn parse_named_type_params<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<Vec<TypeField<'a>>> {
    let lexed = working_set.lex(span, LexOptions::TYPE_PARAMS).map_err(cut)?;
    let mut tokens = Tokens::from_lexed(working_set, &lexed);
    let fields: Vec<Option<TypeField<'a>>> =
        repeat_to_end(alt((keyword(",").value(None), named_type_param.map(Some)))).parse_next(&mut tokens)?;
    Ok(fields.into_iter().flatten().collect())
}

/// One field: `name`, `name: type` or `name,`.
fn named_type_param<'a>(tokens: &mut Tokens<'_, 'a>) -> ParseResult<TypeField<'a>> {
    let working_set = tokens.working_set;
    let not_a_string =
        |span| Diagnostic::message("annotation key not string", span).with_help("a field name must be a string");
    let name = cut_with(item, |tokens| not_a_string(tokens.here())).parse_next(tokens)?;
    let Expr::String(key) = parse_value(working_set, name.span, ExpectedShape::String)?.expr else {
        return Err(cut(not_a_string(name.span)));
    };
    let separator = opt(alt((keyword(":"), keyword(",")))).parse_next(tokens)?;
    let ty = match separator {
        Some(colon) if tokens.text(&colon) == ":" => {
            let ty = cut_with(any, |_| Diagnostic::expected("type after colon", colon.span)).parse_next(tokens)?;
            parse_type(working_set, ty.span)?
        }
        _ => TypeAnnotation { span: name.span.past(), shape: SyntaxShape::Any },
    };
    Ok(TypeField { name: Spanned::new(key.value, name.span), ty })
}
