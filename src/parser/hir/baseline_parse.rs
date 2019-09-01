use crate::context::Context;
use crate::parser::{hir, RawToken, Token};
use crate::Text;
use std::path::PathBuf;

pub fn baseline_parse_single_token(token: &Token, source: &Text) -> hir::Expression {
    match *token.item() {
        RawToken::Number(number) => hir::Expression::number(number.to_number(source), token.span()),
        RawToken::Size(int, unit) => {
            hir::Expression::size(int.to_number(source), unit, token.span())
        }
        RawToken::String(span) => hir::Expression::string(span, token.span()),
        RawToken::Variable(span) if span.slice(source) == "it" => {
            hir::Expression::it_variable(span, token.span())
        }
        RawToken::Variable(span) => hir::Expression::variable(span, token.span()),
        RawToken::External(span) => hir::Expression::external_command(span, token.span()),
        RawToken::Bare => hir::Expression::bare(token.span()),
    }
}

pub fn baseline_parse_token_as_number(token: &Token, source: &Text) -> hir::Expression {
    match *token.item() {
        RawToken::Variable(span) if span.slice(source) == "it" => {
            hir::Expression::it_variable(span, token.span())
        }
        RawToken::External(span) => hir::Expression::external_command(span, token.span()),
        RawToken::Variable(span) => hir::Expression::variable(span, token.span()),
        RawToken::Number(number) => hir::Expression::number(number.to_number(source), token.span()),
        RawToken::Size(number, unit) => {
            hir::Expression::size(number.to_number(source), unit, token.span())
        }
        RawToken::Bare => hir::Expression::bare(token.span()),
        RawToken::String(span) => hir::Expression::string(span, token.span()),
    }
}

pub fn baseline_parse_token_as_string(token: &Token, source: &Text) -> hir::Expression {
    match *token.item() {
        RawToken::Variable(span) if span.slice(source) == "it" => {
            hir::Expression::it_variable(span, token.span())
        }
        RawToken::External(span) => hir::Expression::external_command(span, token.span()),
        RawToken::Variable(span) => hir::Expression::variable(span, token.span()),
        RawToken::Number(_) => hir::Expression::bare(token.span()),
        RawToken::Size(_, _) => hir::Expression::bare(token.span()),
        RawToken::Bare => hir::Expression::bare(token.span()),
        RawToken::String(span) => hir::Expression::string(span, token.span()),
    }
}

pub fn baseline_parse_token_as_path(
    token: &Token,
    context: &Context,
    source: &Text,
) -> hir::Expression {
    match *token.item() {
        RawToken::Variable(span) if span.slice(source) == "it" => {
            hir::Expression::it_variable(span, token.span())
        }
        RawToken::External(span) => hir::Expression::external_command(span, token.span()),
        RawToken::Variable(span) => hir::Expression::variable(span, token.span()),
        RawToken::Number(_) => hir::Expression::bare(token.span()),
        RawToken::Size(_, _) => hir::Expression::bare(token.span()),
        RawToken::Bare => hir::Expression::file_path(
            expand_path(token.span().slice(source), context),
            token.span(),
        ),
        RawToken::String(span) => {
            hir::Expression::file_path(expand_path(span.slice(source), context), token.span())
        }
    }
}

pub fn expand_path(string: &str, context: &Context) -> PathBuf {
    let expanded = shellexpand::tilde_with_context(string, || context.shell_manager.homedir());

    PathBuf::from(expanded.as_ref())
}
