//! Small helpers shared by the parser files (nu-parser's `parse_helpers.rs`):
//! predicates on an item's text (`is_identifier`, `is_variable`, `is_spread`,
//! `is_help_flag`), the interior of a bracketed item (`delimited_interior`), an
//! error constructor (`invalid_literal`) and the placeholders for what failed to
//! parse (`garbage`, `garbage_pipeline`).

use crate::ast::{Expr, Expression, Pipeline, PipelineElement};
use crate::error::{Diagnostic, ErrorKind};
use crate::input::{ParseFailure, ParseResult, cut};
use crate::lex::{Token, TokenContents};
use crate::span::Span;

use super::WorkingSet;

/// Whether `name` can name a variable or parameter: it is not empty and has
/// none of the characters `.[({+-*^%/=!<>&|` (nu's `is_identifier`).
pub fn is_identifier(name: &str) -> bool {
    !name.is_empty()
        && !name.bytes().any(|byte| {
            matches!(
                byte,
                b'.' | b'['
                    | b'('
                    | b'{'
                    | b'+'
                    | b'-'
                    | b'*'
                    | b'^'
                    | b'%'
                    | b'/'
                    | b'='
                    | b'!'
                    | b'<'
                    | b'>'
                    | b'&'
                    | b'|'
            )
        })
}

/// Whether `text` can declare a variable (nu's `is_variable`): `$name` or
/// `name` with an identifier name. A bare `$` qualifies too, as in nu, where it
/// declares a variable that no expression can refer to (`let $ = 1`).
pub fn is_variable(text: &str) -> bool {
    match text.strip_prefix('$') {
        Some(name) if !name.is_empty() => is_identifier(name),
        _ => is_identifier(text),
    }
}

/// The name a variable declaration `text` declares, and its offset in `text`:
/// without the leading `$`, except for a bare `$`.
pub fn declared_variable_name(text: &str) -> (usize, &str) {
    match text.strip_prefix('$') {
        Some(name) if !name.is_empty() => (1, name),
        _ => (0, text),
    }
}

/// `true` for `...x` where `x` starts with one of `heads` (the check in nu's
/// `extract_spread_list` and `extract_spread_record`, whose heads are `[$(` and `{$(`).
pub fn is_spread(text: &str, heads: &[u8]) -> bool {
    text.len() > 3 && text.starts_with("...") && heads.contains(&text.as_bytes()[3])
}

/// The span between the delimiters of an item such as `[...]` or `{...}`,
/// checking that it opens with `open` and closes with `close`.
pub fn delimited_interior(
    working_set: &WorkingSet<'_>,
    span: Span,
    open: &'static str,
    close: &'static str,
) -> ParseResult<Span> {
    let text = working_set.get_span_contents(span);
    if !text.starts_with(open) {
        return Err(cut(Diagnostic::expected(open, Span::new(span.start, span.start + 1))));
    }
    if text.len() < 2 || !text.ends_with(close) {
        return Err(cut(Diagnostic::new(
            ErrorKind::Unclosed { delimiter: close, open: Span::new(span.start, span.start + 1) },
            span.past(),
        )));
    }
    Ok(Span::new(span.start + 1, span.end - 1))
}

/// A literal that was recognised but is malformed (`1..2sec`, `0b2`).
pub fn invalid_literal(kind: &'static str, message: &str, span: Span) -> winnow::error::ErrMode<ParseFailure> {
    cut(Diagnostic::new(ErrorKind::InvalidLiteral { kind, message: message.into() }, span))
}

/// The placeholder expression for text that failed to parse (nu's `garbage`).
pub fn garbage<'a>(span: Span) -> Expression<'a> {
    Expression::new(Expr::Garbage, span)
}

/// A pipeline holding one [`garbage`] expression, standing for a statement
/// that failed to parse (nu's `garbage_pipeline`).
pub fn garbage_pipeline<'a>(span: Span) -> Pipeline<'a> {
    Pipeline {
        span,
        elements: vec![PipelineElement { span, pipe: None, expr: garbage(span), redirection: None }],
        leading_comments: Vec::new(),
        trailing_comments: Vec::new(),
        terminator: None,
    }
}

/// Whether `token` is `--help` or `-h`.
pub fn is_help_flag(working_set: &WorkingSet<'_>, token: &Token) -> bool {
    token.contents == TokenContents::Item && matches!(working_set.get_span_contents(token.span), "--help" | "-h")
}
