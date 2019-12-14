use crate::hir::Expression;
use crate::{
    hir,
    hir::syntax_shape::{
        ExpandSyntax, ExpressionContinuationShape, MemberShape, PathTailShape, PathTailSyntax,
        VariablePathShape,
    },
    hir::tokens_iterator::TokensIterator,
};
use hir::SpannedExpression;
use nu_errors::ParseError;
use nu_source::Span;

#[derive(Debug, Copy, Clone)]
pub struct CoerceBlockShape;

impl ExpandSyntax for CoerceBlockShape {
    type Output = Result<SpannedExpression, ParseError>;

    fn name(&self) -> &'static str {
        "any block"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
    ) -> Result<SpannedExpression, ParseError> {
        // is it just a block?
        token_nodes
            .expand_syntax(BlockShape)
            .or_else(|_| token_nodes.expand_syntax(ShorthandBlockShape))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct BlockShape;

impl ExpandSyntax for BlockShape {
    type Output = Result<SpannedExpression, ParseError>;

    fn name(&self) -> &'static str {
        "block"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
    ) -> Result<SpannedExpression, ParseError> {
        let exprs = token_nodes.block()?;

        Ok(hir::Expression::Block(exprs.item).into_expr(exprs.span))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ShorthandBlockShape;

impl ExpandSyntax for ShorthandBlockShape {
    type Output = Result<SpannedExpression, ParseError>;

    fn name(&self) -> &'static str {
        "shorthand block"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
    ) -> Result<SpannedExpression, ParseError> {
        let mut current = token_nodes.expand_syntax(ShorthandPath)?;
        loop {
            match token_nodes.expand_syntax(ExpressionContinuationShape) {
                Result::Err(_) => break,
                Result::Ok(continuation) => current = continuation.append_to(current),
            }
        }
        let span = current.span;

        let block = hir::Expression::Block(vec![current]).into_expr(span);

        Ok(block)
    }
}

/// A shorthand for `$it.foo."bar"`, used inside of a shorthand block
#[derive(Debug, Copy, Clone)]
pub struct ShorthandPath;

impl ExpandSyntax for ShorthandPath {
    type Output = Result<SpannedExpression, ParseError>;

    fn name(&self) -> &'static str {
        "shorthand path"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
    ) -> Result<SpannedExpression, ParseError> {
        // if it's a variable path, that's the head part
        let path = token_nodes.expand_syntax(VariablePathShape);

        match path {
            Ok(path) => return Ok(path),
            Err(_) => {}
        }

        // Synthesize the head of the shorthand path (`<member>` -> `$it.<member>`)
        let mut head = token_nodes.expand_syntax(ShorthandHeadShape)?;

        // Now that we've synthesized the head, of the path, proceed to expand the tail of the path
        // like any other path.
        let tail = token_nodes.expand_syntax(PathTailShape);

        match tail {
            Err(_) => return Ok(head),
            Ok(PathTailSyntax { tail, span }) => {
                let span = head.span.until(span);

                // For each member that `PathTailShape` expanded, join it onto the existing expression
                // to form a new path
                for member in tail {
                    head = Expression::dot_member(head, member).into_expr(span);
                }

                Ok(head)
            }
        }
    }
}

/// A shorthand for `$it.foo."bar"`, used inside of a shorthand block
#[derive(Debug, Copy, Clone)]
pub struct ShorthandHeadShape;

impl ExpandSyntax for ShorthandHeadShape {
    type Output = Result<SpannedExpression, ParseError>;

    fn name(&self) -> &'static str {
        "shorthand head"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
    ) -> Result<SpannedExpression, ParseError> {
        let head = token_nodes.expand_syntax(MemberShape)?;
        let head = head.to_path_member(&token_nodes.source());

        // Synthesize an `$it` expression
        let it = synthetic_it();
        let span = head.span;

        Ok(Expression::path(it, vec![head]).into_expr(span))
    }
}

fn synthetic_it() -> hir::SpannedExpression {
    Expression::it_variable(Span::unknown()).into_expr(Span::unknown())
}
