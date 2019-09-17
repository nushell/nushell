pub(crate) mod delimited;
pub(crate) mod file_path;
pub(crate) mod list;
pub(crate) mod number;
pub(crate) mod pattern;
pub(crate) mod string;
pub(crate) mod unit;
pub(crate) mod variable_path;

use crate::parser::hir::syntax_shape::{
    expand_expr, expand_syntax, expand_variable, expression::delimited::expand_delimited_expr,
    BareShape, DotShape, ExpandContext, ExpandExpression, ExpandSyntax, ExpressionContinuation,
    ExpressionContinuationShape, UnitShape,
};
use crate::parser::{
    hir,
    hir::{Expression, Operator, TokensIterator},
    RawToken, Token, TokenNode,
};
use crate::prelude::*;
use std::path::PathBuf;

#[derive(Debug, Copy, Clone)]
pub struct AnyExpressionShape;

impl ExpandExpression for AnyExpressionShape {
    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
        context: &ExpandContext,
    ) -> Result<hir::Expression, ShellError> {
        // Look for an expression at the cursor
        let head = expand_expr(&AnyExpressionStartShape, token_nodes, context)?;

        continue_expression(head, token_nodes, context)
    }
}

pub(crate) fn continue_expression(
    mut head: hir::Expression,
    token_nodes: &mut TokensIterator<'_>,
    context: &ExpandContext,
) -> Result<hir::Expression, ShellError> {
    loop {
        // Check to see whether there's any continuation after the head expression
        let continuation = expand_syntax(&ExpressionContinuationShape, token_nodes, context);

        match continuation {
            // If there's no continuation, return the head
            Err(_) => return Ok(head),
            // Otherwise, form a new expression by combining the head with the continuation
            Ok(continuation) => match continuation {
                // If the continuation is a `.member`, form a path with the new member
                ExpressionContinuation::DotSuffix(_dot, member) => {
                    head = Expression::dot_member(head, member);
                }

                // Otherwise, if the continuation is an infix suffix, form an infix expression
                ExpressionContinuation::InfixSuffix(op, expr) => {
                    head = Expression::infix(head, op, expr);
                }
            },
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct AnyExpressionStartShape;

impl ExpandExpression for AnyExpressionStartShape {
    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
        context: &ExpandContext,
    ) -> Result<hir::Expression, ShellError> {
        let size = expand_expr(&UnitShape, token_nodes, context);

        match size {
            Ok(expr) => return Ok(expr),
            Err(_) => {}
        }

        let peek_next = token_nodes.peek_any().not_eof("expression")?;

        let head = match peek_next.node {
            TokenNode::Token(token) => match token.item {
                RawToken::Bare | RawToken::Operator(Operator::Dot) => {
                    let start = token.tag;
                    peek_next.commit();

                    let end = expand_syntax(&BareTailShape, token_nodes, context)?;

                    match end {
                        Some(end) => return Ok(hir::Expression::bare(start.until(end))),
                        None => return Ok(hir::Expression::bare(start)),
                    }
                }
                _ => {
                    peek_next.commit();
                    expand_one_context_free_token(*token, context)
                }
            },
            node @ TokenNode::Call(_)
            | node @ TokenNode::Nodes(_)
            | node @ TokenNode::Pipeline(_)
            | node @ TokenNode::Flag(_)
            | node @ TokenNode::Member(_)
            | node @ TokenNode::Whitespace(_) => {
                return Err(ShellError::type_error(
                    "expression",
                    node.tagged_type_name(),
                ))
            }
            TokenNode::Delimited(delimited) => {
                peek_next.commit();
                expand_delimited_expr(delimited, context)
            }

            TokenNode::Error(error) => return Err(*error.item.clone()),
        }?;

        Ok(head)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct BareTailShape;

impl ExpandSyntax for BareTailShape {
    type Output = Option<Tag>;

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Option<Tag>, ShellError> {
        let mut end: Option<Tag> = None;

        loop {
            match expand_syntax(&BareShape, token_nodes, context) {
                Ok(bare) => {
                    end = Some(bare.tag);
                    continue;
                }

                Err(_) => match expand_syntax(&DotShape, token_nodes, context) {
                    Ok(dot) => {
                        end = Some(dot);
                        continue;
                    }

                    Err(_) => break,
                },
            }
        }

        Ok(end)
    }
}

fn expand_one_context_free_token<'a, 'b>(
    token: Token,
    context: &ExpandContext,
) -> Result<hir::Expression, ShellError> {
    Ok(match token.item {
        RawToken::Number(number) => {
            hir::Expression::number(number.to_number(context.source), token.tag)
        }
        RawToken::Operator(..) => {
            return Err(ShellError::syntax_error(
                "unexpected operator, expected an expression".tagged(token.tag),
            ))
        }
        RawToken::Size(..) => unimplemented!("size"),
        RawToken::String(tag) => hir::Expression::string(tag, token.tag),
        RawToken::Variable(tag) => expand_variable(tag, token.tag, &context.source),
        RawToken::ExternalCommand(_) => unimplemented!(),
        RawToken::ExternalWord => unimplemented!(),
        RawToken::GlobPattern => hir::Expression::pattern(token.tag),
        RawToken::Bare => hir::Expression::string(token.tag, token.tag),
    })
}

pub fn expand_file_path(string: &str, context: &ExpandContext) -> PathBuf {
    let expanded = shellexpand::tilde_with_context(string, || context.homedir());

    PathBuf::from(expanded.as_ref())
}
