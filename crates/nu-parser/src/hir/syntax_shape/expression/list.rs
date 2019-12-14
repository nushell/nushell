use crate::hir::syntax_shape::flat_shape::FlatShape;
use crate::{
    hir,
    hir::syntax_shape::{AnyExpressionShape, ExpandSyntax, MaybeSpaceShape},
    hir::TokensIterator,
};
use derive_new::new;
use nu_source::{b, DebugDocBuilder, HasSpan, PrettyDebugWithSource, Span, Spanned, SpannedItem};

#[derive(Debug, Clone)]
pub struct ExpressionListSyntax {
    pub exprs: Spanned<Vec<hir::SpannedExpression>>,
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

    fn expand<'a, 'b>(&self, token_nodes: &mut TokensIterator<'_>) -> ExpressionListSyntax {
        // We encountered a parsing error and will continue with simpler coloring ("backoff
        // coloring mode")
        let mut backoff = false;

        let mut exprs = vec![];

        let start = token_nodes.span_at_cursor();

        token_nodes.expand_infallible(MaybeSpaceShape);

        if token_nodes.at_end() {
            return ExpressionListSyntax {
                exprs: exprs.spanned(start),
            };
        }

        let expr = token_nodes.expand_syntax(AnyExpressionShape);

        match expr {
            Ok(expr) => exprs.push(expr),
            Err(_) => backoff = true,
        }

        loop {
            if token_nodes.at_end() {
                let end = token_nodes.span_at_cursor();
                return ExpressionListSyntax {
                    exprs: exprs.spanned(start.until(end)),
                };
            }

            if backoff {
                let len = token_nodes.state().shapes().len();

                // If we previously encountered a parsing error, use backoff coloring mode
                token_nodes
                    .expand_infallible(SimplestExpression::new(vec!["expression".to_string()]));

                if len == token_nodes.state().shapes().len() && !token_nodes.at_end() {
                    // This should never happen, but if it does, a panic is better than an infinite loop
                    panic!("Unexpected tokens left that couldn't be colored even with SimplestExpression")
                }
            } else {
                let expr = token_nodes.atomic_parse(|token_nodes| {
                    token_nodes.expand_infallible(MaybeSpaceShape);
                    token_nodes.expand_syntax(AnyExpressionShape)
                });

                match expr {
                    Ok(expr) => exprs.push(expr),
                    Err(_) => {
                        backoff = true;
                    }
                }
            }
        }
    }
}

/// BackoffColoringMode consumes all of the remaining tokens in an infallible way
#[derive(Debug, Clone, new)]
pub struct BackoffColoringMode {
    allowed: Vec<String>,
}

impl ExpandSyntax for BackoffColoringMode {
    type Output = Option<Span>;

    fn name(&self) -> &'static str {
        "BackoffColoringMode"
    }

    fn expand<'a, 'b>(&self, token_nodes: &'b mut TokensIterator<'a>) -> Option<Span> {
        loop {
            if token_nodes.at_end() {
                break;
            }

            let len = token_nodes.state().shapes().len();
            token_nodes.expand_infallible(SimplestExpression::new(self.allowed.clone()));

            if len == token_nodes.state().shapes().len() && !token_nodes.at_end() {
                // This shouldn't happen, but if it does, a panic is better than an infinite loop
                panic!("SimplestExpression failed to consume any tokens, but it's not at the end. This is unexpected\n== token nodes==\n{:#?}\n\n== shapes ==\n{:#?}", token_nodes, token_nodes.state().shapes());
            }
        }

        None
    }
}

/// The point of `SimplestExpression` is to serve as an infallible base case for coloring.
/// As a last ditch effort, if we can't find any way to parse the head of the stream as an
/// expression, fall back to simple coloring.
#[derive(Debug, Clone, new)]
pub struct SimplestExpression {
    valid_shapes: Vec<String>,
}

impl ExpandSyntax for SimplestExpression {
    type Output = Span;

    fn name(&self) -> &'static str {
        "SimplestExpression"
    }

    fn expand<'a, 'b>(&self, token_nodes: &'b mut TokensIterator<'a>) -> Span {
        if token_nodes.at_end() {
            return Span::unknown();
        }

        let source = token_nodes.source();

        let peeked = token_nodes.peek();

        match peeked.not_eof("simplest expression") {
            Err(_) => token_nodes.span_at_cursor(),
            Ok(peeked) => {
                let token = peeked.commit();

                for shape in FlatShape::shapes(token, &source) {
                    token_nodes.color_err(shape, self.valid_shapes.clone())
                }

                token.span()
            }
        }
    }
}
