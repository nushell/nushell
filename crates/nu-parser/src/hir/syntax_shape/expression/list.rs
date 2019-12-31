use crate::{
    hir,
    hir::syntax_shape::{
        color_fallible_syntax, color_syntax, expand_atom, expand_expr, maybe_spaced, spaced,
        AnyExpressionShape, ColorSyntax, ExpandContext, ExpandSyntax, ExpansionRule,
        MaybeSpaceShape, SpaceShape,
    },
    hir::TokensIterator,
};
use nu_errors::ParseError;
use nu_source::{b, DebugDocBuilder, HasSpan, PrettyDebugWithSource, Span, Spanned, SpannedItem};

#[derive(Debug, Clone)]
pub struct ExpressionListSyntax {
    pub exprs: Spanned<Vec<hir::Expression>>,
}

impl HasSpan for ExpressionListSyntax {
    fn span(&self) -> Span {
        self.exprs.span
    }
}

impl PrettyDebugWithSource for ExpressionListSyntax {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        b::intersperse(
            self.exprs.iter().map(|e| e.pretty_debug(source)),
            b::space(),
        )
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ExpressionListShape;

impl ExpandSyntax for ExpressionListShape {
    type Output = ExpressionListSyntax;

    fn name(&self) -> &'static str {
        "expression list"
    }

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
        context: &ExpandContext,
    ) -> Result<ExpressionListSyntax, ParseError> {
        let mut exprs = vec![];

        let start = token_nodes.span_at_cursor();

        if token_nodes.at_end_possible_ws() {
            return Ok(ExpressionListSyntax {
                exprs: exprs.spanned(start),
            });
        }

        let expr = expand_expr(&maybe_spaced(AnyExpressionShape), token_nodes, context)?;

        exprs.push(expr);

        loop {
            if token_nodes.at_end_possible_ws() {
                let end = token_nodes.span_at_cursor();
                return Ok(ExpressionListSyntax {
                    exprs: exprs.spanned(start.until(end)),
                });
            }

            let expr = expand_expr(&spaced(AnyExpressionShape), token_nodes, context)?;

            exprs.push(expr);
        }
    }
}

impl ColorSyntax for ExpressionListShape {
    type Info = ();
    type Input = ();

    fn name(&self) -> &'static str {
        "ExpressionListShape"
    }

    /// The intent of this method is to fully color an expression list shape infallibly.
    /// This means that if we can't expand a token into an expression, we fall back to
    /// a simpler coloring strategy.
    ///
    /// This would apply to something like `where x >`, which includes an incomplete
    /// binary operator. Since we will fail to process it as a binary operator, we'll
    /// fall back to a simpler coloring and move on.
    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) {
        // We encountered a parsing error and will continue with simpler coloring ("backoff
        // coloring mode")
        let mut backoff = false;

        // Consume any leading whitespace
        color_syntax(&MaybeSpaceShape, token_nodes, context);

        loop {
            // If we reached the very end of the token stream, we're done
            if token_nodes.at_end() {
                return;
            }

            if backoff {
                let len = token_nodes.state().shapes().len();

                // If we previously encountered a parsing error, use backoff coloring mode
                color_syntax(&SimplestExpression, token_nodes, context);

                if len == token_nodes.state().shapes().len() && !token_nodes.at_end() {
                    // This should never happen, but if it does, a panic is better than an infinite loop
                    panic!("Unexpected tokens left that couldn't be colored even with SimplestExpression")
                }
            } else {
                // Try to color the head of the stream as an expression
                if color_fallible_syntax(&AnyExpressionShape, token_nodes, context).is_err() {
                    // If no expression was found, switch to backoff coloring mode

                    backoff = true;
                    continue;
                }

                // If an expression was found, consume a space
                if color_fallible_syntax(&SpaceShape, token_nodes, context).is_err() {
                    // If no space was found, we're either at the end or there's an error.
                    // Either way, switch to backoff coloring mode. If we're at the end
                    // it won't have any consequences.
                    backoff = true;
                }
                // Otherwise, move on to the next expression
            }
        }
    }
}

/// BackoffColoringMode consumes all of the remaining tokens in an infallible way
#[derive(Debug, Copy, Clone)]
pub struct BackoffColoringMode;

impl ColorSyntax for BackoffColoringMode {
    type Info = ();
    type Input = ();

    fn name(&self) -> &'static str {
        "BackoffColoringMode"
    }

    fn color_syntax<'a, 'b>(
        &self,
        _input: &Self::Input,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Self::Info {
        loop {
            if token_nodes.at_end() {
                break;
            }

            let len = token_nodes.state().shapes().len();
            color_syntax(&SimplestExpression, token_nodes, context);

            if len == token_nodes.state().shapes().len() && !token_nodes.at_end() {
                // This shouldn't happen, but if it does, a panic is better than an infinite loop
                panic!("SimplestExpression failed to consume any tokens, but it's not at the end. This is unexpected\n== token nodes==\n{:#?}\n\n== shapes ==\n{:#?}", token_nodes, token_nodes.state().shapes());
            }
        }
    }
}

/// The point of `SimplestExpression` is to serve as an infallible base case for coloring.
/// As a last ditch effort, if we can't find any way to parse the head of the stream as an
/// expression, fall back to simple coloring.
#[derive(Debug, Copy, Clone)]
pub struct SimplestExpression;

impl ColorSyntax for SimplestExpression {
    type Info = ();
    type Input = ();

    fn name(&self) -> &'static str {
        "SimplestExpression"
    }

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) {
        let atom = expand_atom(
            token_nodes,
            "any token",
            context,
            ExpansionRule::permissive(),
        );

        match atom {
            Err(_) => {}
            Ok(atom) => token_nodes.mutate_shapes(|shapes| atom.color_tokens(shapes)),
        }
    }
}
