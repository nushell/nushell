pub(crate) mod atom;
pub(crate) mod delimited;
pub(crate) mod file_path;
pub(crate) mod list;
pub(crate) mod number;
pub(crate) mod pattern;
pub(crate) mod range;
pub(crate) mod string;
pub(crate) mod unit;
pub(crate) mod variable_path;

use crate::hir::syntax_shape::{
    color_delimited_square, color_fallible_syntax, color_fallible_syntax_with, expand_atom,
    expand_delimited_square, expand_expr, expand_syntax, BareShape, ColorableDotShape, DotShape,
    ExpandContext, ExpandExpression, ExpandSyntax, ExpansionRule, ExpressionContinuation,
    ExpressionContinuationShape, FallibleColorSyntax, FlatShape, UnspannedAtomicToken,
};
use crate::{
    hir,
    hir::{Expression, TokensIterator},
};
use nu_errors::{ParseError, ShellError};
use nu_source::{HasSpan, Span, Spanned, SpannedItem, Tag};
use std::path::PathBuf;

#[derive(Debug, Copy, Clone)]
pub struct AnyExpressionShape;

impl ExpandExpression for AnyExpressionShape {
    fn name(&self) -> &'static str {
        "any expression"
    }

    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
        context: &ExpandContext,
    ) -> Result<hir::Expression, ParseError> {
        // Look for an expression at the cursor
        let head = expand_expr(&AnyExpressionStartShape, token_nodes, context)?;

        Ok(continue_expression(head, token_nodes, context))
    }
}

impl FallibleColorSyntax for AnyExpressionShape {
    type Info = ();
    type Input = ();

    fn name(&self) -> &'static str {
        "AnyExpressionShape"
    }

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<(), ShellError> {
        // Look for an expression at the cursor
        color_fallible_syntax(&AnyExpressionStartShape, token_nodes, context)?;

        match continue_coloring_expression(token_nodes, context) {
            Err(_) => {
                // it's fine for there to be no continuation
            }

            Ok(()) => {}
        }

        Ok(())
    }
}

pub(crate) fn continue_expression(
    mut head: hir::Expression,
    token_nodes: &mut TokensIterator<'_>,
    context: &ExpandContext,
) -> hir::Expression {
    loop {
        // Check to see whether there's any continuation after the head expression
        let continuation = expand_syntax(&ExpressionContinuationShape, token_nodes, context);

        match continuation {
            // If there's no continuation, return the head
            Err(_) => return head,
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

pub(crate) fn continue_coloring_expression(
    token_nodes: &mut TokensIterator<'_>,
    context: &ExpandContext,
) -> Result<(), ShellError> {
    // if there's not even one expression continuation, fail
    color_fallible_syntax(&ExpressionContinuationShape, token_nodes, context)?;

    loop {
        // Check to see whether there's any continuation after the head expression
        let result = color_fallible_syntax(&ExpressionContinuationShape, token_nodes, context);

        if result.is_err() {
            // We already saw one continuation, so just return
            return Ok(());
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct AnyExpressionStartShape;

impl ExpandExpression for AnyExpressionStartShape {
    fn name(&self) -> &'static str {
        "any expression start"
    }

    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
        context: &ExpandContext,
    ) -> Result<hir::Expression, ParseError> {
        let atom = expand_atom(token_nodes, "expression", context, ExpansionRule::new())?;

        match atom.unspanned {
            UnspannedAtomicToken::Size { number, unit } => Ok(hir::Expression::size(
                number.to_number(context.source),
                unit.item,
                Tag {
                    span: atom.span,
                    anchor: None,
                },
            )),

            UnspannedAtomicToken::SquareDelimited { nodes, .. } => {
                expand_delimited_square(&nodes, atom.span, context)
            }

            UnspannedAtomicToken::Word { .. } => {
                let end = expand_syntax(&BareTailShape, token_nodes, context)?;
                Ok(hir::Expression::bare(atom.span.until_option(end)))
            }

            other => other
                .into_atomic_token(atom.span)
                .to_hir(context, "expression"),
        }
    }
}

impl FallibleColorSyntax for AnyExpressionStartShape {
    type Info = ();
    type Input = ();

    fn name(&self) -> &'static str {
        "AnyExpressionStartShape"
    }

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<(), ShellError> {
        let atom = token_nodes.spanned(|token_nodes| {
            expand_atom(
                token_nodes,
                "expression",
                context,
                ExpansionRule::permissive(),
            )
        });

        let atom = match atom {
            Spanned {
                item: Err(_err),
                span,
            } => {
                token_nodes.color_shape(FlatShape::Error.spanned(span));
                return Ok(());
            }

            Spanned {
                item: Ok(value), ..
            } => value,
        };

        match atom.unspanned {
            UnspannedAtomicToken::Size { number, unit } => token_nodes.color_shape(
                FlatShape::Size {
                    number: number.span(),
                    unit: unit.span,
                }
                .spanned(atom.span),
            ),

            UnspannedAtomicToken::SquareDelimited { nodes, spans } => {
                token_nodes.child(
                    (&nodes[..]).spanned(atom.span),
                    context.source.clone(),
                    |tokens| {
                        color_delimited_square(spans, tokens, atom.span, context);
                    },
                );
            }

            UnspannedAtomicToken::Word { .. } | UnspannedAtomicToken::Dot { .. } => {
                token_nodes.color_shape(FlatShape::Word.spanned(atom.span));
            }

            _ => token_nodes.mutate_shapes(|shapes| atom.color_tokens(shapes)),
        }

        Ok(())
    }
}

#[derive(Debug, Copy, Clone)]
pub struct BareTailShape;

impl FallibleColorSyntax for BareTailShape {
    type Info = ();
    type Input = ();

    fn name(&self) -> &'static str {
        "BareTailShape"
    }

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<(), ShellError> {
        let len = token_nodes.state().shapes().len();

        loop {
            let word =
                color_fallible_syntax_with(&BareShape, &FlatShape::Word, token_nodes, context);

            if word.is_ok() {
                // if a word was found, continue
                continue;
            }

            // if a word wasn't found, try to find a dot

            // try to find a dot
            let dot = color_fallible_syntax_with(
                &ColorableDotShape,
                &FlatShape::Word,
                token_nodes,
                context,
            );

            match dot {
                // if a dot was found, try to find another word
                Ok(_) => continue,
                // otherwise, we're done
                Err(_) => break,
            }
        }

        if token_nodes.state().shapes().len() > len {
            Ok(())
        } else {
            Err(ShellError::syntax_error(
                "No tokens matched BareTailShape".spanned_unknown(),
            ))
        }
    }
}

impl ExpandSyntax for BareTailShape {
    fn name(&self) -> &'static str {
        "word continuation"
    }

    type Output = Option<Span>;

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Option<Span>, ParseError> {
        let mut end: Option<Span> = None;

        loop {
            match expand_syntax(&BareShape, token_nodes, context) {
                Ok(bare) => {
                    end = Some(bare.span);
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

pub fn expand_file_path(string: &str, context: &ExpandContext) -> PathBuf {
    let expanded = shellexpand::tilde_with_context(string, || context.homedir());

    PathBuf::from(expanded.as_ref())
}
