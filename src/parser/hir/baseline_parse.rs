use crate::context::Context;
use crate::errors::ShellError;
use crate::parser::{hir, RawToken, Token};
use crate::TaggedItem;
use crate::Text;
use std::path::PathBuf;

pub fn baseline_parse_single_token(
    token: &Token,
    source: &Text,
) -> Result<hir::Expression, ShellError> {
    Ok(match *token.item() {
        RawToken::Number(number) => hir::Expression::number(number.to_number(source), token.tag()),
        RawToken::Size(int, unit) => {
            hir::Expression::size(int.to_number(source), unit, token.tag())
        }
        RawToken::String(tag) => hir::Expression::string(tag, token.tag()),
        RawToken::Variable(tag) if tag.slice(source) == "it" => {
            hir::Expression::it_variable(tag, token.tag())
        }
        RawToken::Variable(tag) => hir::Expression::variable(tag, token.tag()),
        RawToken::ExternalCommand(tag) => hir::Expression::external_command(tag, token.tag()),
        RawToken::ExternalWord => return Err(ShellError::invalid_external_word(token.tag())),
        RawToken::GlobPattern => hir::Expression::pattern(token.tag()),
        RawToken::Bare => hir::Expression::bare(token.tag()),
    })
}

pub fn baseline_parse_token_as_number(
    token: &Token,
    source: &Text,
) -> Result<hir::Expression, ShellError> {
    Ok(match *token.item() {
        RawToken::Variable(tag) if tag.slice(source) == "it" => {
            hir::Expression::it_variable(tag, token.tag())
        }
        RawToken::ExternalCommand(tag) => hir::Expression::external_command(tag, token.tag()),
        RawToken::ExternalWord => return Err(ShellError::invalid_external_word(token.tag())),
        RawToken::Variable(tag) => hir::Expression::variable(tag, token.tag()),
        RawToken::Number(number) => hir::Expression::number(number.to_number(source), token.tag()),
        RawToken::Size(number, unit) => {
            hir::Expression::size(number.to_number(source), unit, token.tag())
        }
        RawToken::Bare => hir::Expression::bare(token.tag()),
        RawToken::GlobPattern => {
            return Err(ShellError::type_error(
                "Number",
                "glob pattern".to_string().tagged(token.tag()),
            ))
        }
        RawToken::String(tag) => hir::Expression::string(tag, token.tag()),
    })
}

pub fn baseline_parse_token_as_string(
    token: &Token,
    source: &Text,
) -> Result<hir::Expression, ShellError> {
    Ok(match *token.item() {
        RawToken::Variable(tag) if tag.slice(source) == "it" => {
            hir::Expression::it_variable(tag, token.tag())
        }
        RawToken::ExternalCommand(tag) => hir::Expression::external_command(tag, token.tag()),
        RawToken::ExternalWord => return Err(ShellError::invalid_external_word(token.tag())),
        RawToken::Variable(tag) => hir::Expression::variable(tag, token.tag()),
        RawToken::Number(_) => hir::Expression::bare(token.tag()),
        RawToken::Size(_, _) => hir::Expression::bare(token.tag()),
        RawToken::Bare => hir::Expression::bare(token.tag()),
        RawToken::GlobPattern => {
            return Err(ShellError::type_error(
                "String",
                "glob pattern".tagged(token.tag()),
            ))
        }
        RawToken::String(tag) => hir::Expression::string(tag, token.tag()),
    })
}

pub fn baseline_parse_token_as_path(
    token: &Token,
    context: &Context,
    source: &Text,
) -> Result<hir::Expression, ShellError> {
    Ok(match *token.item() {
        RawToken::Variable(tag) if tag.slice(source) == "it" => {
            hir::Expression::it_variable(tag, token.tag())
        }
        RawToken::ExternalCommand(tag) => hir::Expression::external_command(tag, token.tag()),
        RawToken::ExternalWord => return Err(ShellError::invalid_external_word(token.tag())),
        RawToken::Variable(tag) => hir::Expression::variable(tag, token.tag()),
        RawToken::Number(_) => hir::Expression::bare(token.tag()),
        RawToken::Size(_, _) => hir::Expression::bare(token.tag()),
        RawToken::Bare => {
            hir::Expression::file_path(expand_path(token.tag().slice(source), context), token.tag())
        }
        RawToken::GlobPattern => {
            return Err(ShellError::type_error(
                "Path",
                "glob pattern".tagged(token.tag()),
            ))
        }
        RawToken::String(tag) => {
            hir::Expression::file_path(expand_path(tag.slice(source), context), token.tag())
        }
    })
}

pub fn baseline_parse_token_as_pattern(
    token: &Token,
    context: &Context,
    source: &Text,
) -> Result<hir::Expression, ShellError> {
    Ok(match *token.item() {
        RawToken::Variable(tag) if tag.slice(source) == "it" => {
            hir::Expression::it_variable(tag, token.tag())
        }
        RawToken::ExternalCommand(_) => {
            return Err(ShellError::syntax_error(
                "Invalid external command".to_string().tagged(token.tag()),
            ))
        }
        RawToken::ExternalWord => return Err(ShellError::invalid_external_word(token.tag())),
        RawToken::Variable(tag) => hir::Expression::variable(tag, token.tag()),
        RawToken::Number(_) => hir::Expression::bare(token.tag()),
        RawToken::Size(_, _) => hir::Expression::bare(token.tag()),
        RawToken::GlobPattern => hir::Expression::pattern(token.tag()),
        RawToken::Bare => {
            hir::Expression::file_path(expand_path(token.tag().slice(source), context), token.tag())
        }
        RawToken::String(tag) => {
            hir::Expression::file_path(expand_path(tag.slice(source), context), token.tag())
        }
    })
}

pub fn expand_path(string: &str, context: &Context) -> PathBuf {
    let expanded = shellexpand::tilde_with_context(string, || context.shell_manager.homedir());

    PathBuf::from(expanded.as_ref())
}
