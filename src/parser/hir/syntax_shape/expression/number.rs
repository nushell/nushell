use crate::parser::hir::syntax_shape::{parse_single_node, ExpandContext, ExpandExpression};
use crate::parser::{
    hir,
    hir::{RawNumber, TokensIterator},
    RawToken,
};
use crate::prelude::*;

#[derive(Debug, Copy, Clone)]
pub struct NumberShape;

impl ExpandExpression for NumberShape {
    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
        context: &ExpandContext,
    ) -> Result<hir::Expression, ShellError> {
        parse_single_node(token_nodes, "Number", |token, token_tag| {
            Ok(match token {
                RawToken::GlobPattern => {
                    return Err(ShellError::type_error(
                        "Number",
                        "glob pattern".to_string().tagged(token_tag),
                    ))
                }
                RawToken::Operator(..) => {
                    return Err(ShellError::type_error(
                        "Number",
                        "operator".to_string().tagged(token_tag),
                    ))
                }
                RawToken::Variable(tag) if tag.slice(context.source) == "it" => {
                    hir::Expression::it_variable(tag, token_tag)
                }
                RawToken::ExternalCommand(tag) => hir::Expression::external_command(tag, token_tag),
                RawToken::ExternalWord => return Err(ShellError::invalid_external_word(token_tag)),
                RawToken::Variable(tag) => hir::Expression::variable(tag, token_tag),
                RawToken::Number(number) => {
                    hir::Expression::number(number.to_number(context.source), token_tag)
                }
                RawToken::Size(number, unit) => {
                    hir::Expression::size(number.to_number(context.source), unit, token_tag)
                }
                RawToken::Bare => hir::Expression::bare(token_tag),
                RawToken::String(tag) => hir::Expression::string(tag, token_tag),
            })
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub struct IntShape;

impl ExpandExpression for IntShape {
    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
        context: &ExpandContext,
    ) -> Result<hir::Expression, ShellError> {
        parse_single_node(token_nodes, "Integer", |token, token_tag| {
            Ok(match token {
                RawToken::GlobPattern => {
                    return Err(ShellError::type_error(
                        "Integer",
                        "glob pattern".to_string().tagged(token_tag),
                    ))
                }
                RawToken::Operator(..) => {
                    return Err(ShellError::type_error(
                        "Integer",
                        "operator".to_string().tagged(token_tag),
                    ))
                }
                RawToken::Variable(tag) if tag.slice(context.source) == "it" => {
                    hir::Expression::it_variable(tag, token_tag)
                }
                RawToken::ExternalCommand(tag) => hir::Expression::external_command(tag, token_tag),
                RawToken::ExternalWord => return Err(ShellError::invalid_external_word(token_tag)),
                RawToken::Variable(tag) => hir::Expression::variable(tag, token_tag),
                RawToken::Number(number @ RawNumber::Int(_)) => {
                    hir::Expression::number(number.to_number(context.source), token_tag)
                }
                token @ RawToken::Number(_) => {
                    return Err(ShellError::type_error(
                        "Integer",
                        token.type_name().tagged(token_tag),
                    ));
                }
                RawToken::Size(number, unit) => {
                    hir::Expression::size(number.to_number(context.source), unit, token_tag)
                }
                RawToken::Bare => hir::Expression::bare(token_tag),
                RawToken::String(tag) => hir::Expression::string(tag, token_tag),
            })
        })
    }
}
