use crate::parser::{hir, RawToken, Token};
use crate::Text;

pub fn baseline_parse_single_token(token: &Token, source: &Text) -> hir::Expression {
    match *token.item() {
        RawToken::Integer(int) => hir::Expression::int(int, token.span),
        RawToken::Size(int, unit) => hir::Expression::size(int, unit, token.span),
        RawToken::String(span) => hir::Expression::string(span, token.span),
        RawToken::Variable(span) if span.slice(source) == "it" => {
            hir::Expression::it_variable(span, token.span)
        }
        RawToken::Variable(span) => hir::Expression::variable(span, token.span),
        RawToken::Bare => hir::Expression::bare(token.span),
    }
}

pub fn baseline_parse_token_as_string(token: &Token, source: &Text) -> hir::Expression {
    match *token.item() {
        RawToken::Variable(span) if span.slice(source) == "it" => {
            hir::Expression::it_variable(span, token.span)
        }
        RawToken::Variable(span) => hir::Expression::variable(span, token.span),
        RawToken::Integer(_) => hir::Expression::bare(token.span),
        RawToken::Size(_, _) => hir::Expression::bare(token.span),
        RawToken::Bare => hir::Expression::bare(token.span),
        RawToken::String(span) => hir::Expression::string(span, token.span),
    }
}
