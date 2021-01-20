use nu_errors::ParseError;
use nu_protocol::hir::{Expression, SpannedExpression};
use nu_source::{Span, Spanned, SpannedItem};

use crate::lex::lexer::Token;

pub(crate) fn token_to_spanned_string(token: &Token) -> Spanned<String> {
    token.contents.to_string().spanned(token.span)
}

/// Easy shorthand function to create a garbage expression at the given span
pub fn garbage(span: Span) -> SpannedExpression {
    SpannedExpression::new(Expression::Garbage, span)
}

pub(crate) fn trim_quotes(input: &str) -> String {
    let mut chars = input.chars();

    match (chars.next(), chars.next_back()) {
        (Some('\''), Some('\'')) => chars.collect(),
        (Some('"'), Some('"')) => chars.collect(),
        (Some('`'), Some('`')) => chars.collect(),
        _ => input.to_string(),
    }
}

pub(crate) fn verify_and_strip(
    contents: &Spanned<String>,
    left: char,
    right: char,
) -> (String, Option<ParseError>) {
    let mut chars = contents.item.chars();

    match (chars.next(), chars.next_back()) {
        (Some(l), Some(r)) if l == left && r == right => {
            let output: String = chars.collect();
            (output, None)
        }
        _ => (
            String::new(),
            Some(ParseError::mismatch(
                format!("value in {} {}", left, right),
                contents.clone(),
            )),
        ),
    }
}
