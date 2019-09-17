use crate::errors::ShellError;
use crate::parser::{
    hir,
    hir::syntax_shape::{
        continue_expression, expand_expr, expand_syntax, ExpandContext, ExpandExpression,
        ExpressionListShape, PathTailShape, VariablePathShape,
    },
    hir::tokens_iterator::TokensIterator,
    RawToken, TokenNode,
};
use crate::{Tag, Tagged, TaggedItem};

#[derive(Debug, Copy, Clone)]
pub struct AnyBlockShape;

impl ExpandExpression for AnyBlockShape {
    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
        context: &ExpandContext,
    ) -> Result<hir::Expression, ShellError> {
        let block = token_nodes.peek_non_ws().not_eof("block")?;

        // is it just a block?
        let block = block.node.as_block();

        match block {
            Some(block) => {
                let mut iterator = TokensIterator::new(&block.item, context.tag, false);

                let exprs = expand_syntax(&ExpressionListShape, &mut iterator, context)?;

                return Ok(hir::RawExpression::Block(exprs).tagged(block.tag));
            }
            _ => {}
        }

        expand_syntax(&ShorthandBlock, token_nodes, context)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ShorthandBlock;

impl ExpandExpression for ShorthandBlock {
    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<hir::Expression, ShellError> {
        let path = expand_expr(&ShorthandPath, token_nodes, context)?;
        let start = path.tag;
        let expr = continue_expression(path, token_nodes, context)?;
        let end = expr.tag;
        let block = hir::RawExpression::Block(vec![expr]).tagged(start.until(end));

        Ok(block)
    }
}

/// A shorthand for `$it.foo."bar"`, used inside of a shorthand block
#[derive(Debug, Copy, Clone)]
pub struct ShorthandPath;

impl ExpandExpression for ShorthandPath {
    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<hir::Expression, ShellError> {
        // if it's a variable path, that's the head part
        let path = expand_expr(&VariablePathShape, token_nodes, context);

        match path {
            Ok(path) => return Ok(path),
            Err(_) => {}
        }

        // Synthesize the head of the shorthand path (`<member>` -> `$it.<member>`)
        let mut head = expand_expr(&ShorthandHeadShape, token_nodes, context)?;

        // Now that we've synthesized the head, of the path, proceed to expand the tail of the path
        // like any other path.
        let tail = expand_syntax(&PathTailShape, token_nodes, context);

        match tail {
            Err(_) => return Ok(head),
            Ok((tail, _)) => {
                // For each member that `PathTailShape` expanded, join it onto the existing expression
                // to form a new path
                for member in tail {
                    head = hir::Expression::dot_member(head, member);
                }

                println!("{:?}", head);

                Ok(head)
            }
        }
    }
}

/// A shorthand for `$it.foo."bar"`, used inside of a shorthand block
#[derive(Debug, Copy, Clone)]
pub struct ShorthandHeadShape;

impl ExpandExpression for ShorthandHeadShape {
    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<hir::Expression, ShellError> {
        // A shorthand path must not be at EOF
        let peeked = token_nodes.peek_non_ws().not_eof("shorthand path")?;

        match peeked.node {
            // If the head of a shorthand path is a bare token, it expands to `$it.bare`
            TokenNode::Token(Tagged {
                item: RawToken::Bare,
                tag,
            }) => {
                // Commit the peeked token
                peeked.commit();

                // Synthesize an `$it` expression
                let it = synthetic_it(token_nodes.anchor());

                // Make a path out of `$it` and the bare token as a member
                Ok(hir::Expression::path(
                    it,
                    vec![tag.tagged_string(context.source)],
                    tag,
                ))
            }

            // If the head of a shorthand path is a string, it expands to `$it."some string"`
            TokenNode::Token(Tagged {
                item: RawToken::String(inner),
                tag: outer,
            }) => {
                // Commit the peeked token
                peeked.commit();

                // Synthesize an `$it` expression
                let it = synthetic_it(token_nodes.anchor());

                // Make a path out of `$it` and the bare token as a member
                Ok(hir::Expression::path(
                    it,
                    vec![inner.string(context.source).tagged(outer)],
                    outer,
                ))
            }

            // Any other token is not a valid bare head
            other => {
                return Err(ShellError::type_error(
                    "shorthand path",
                    other.tagged_type_name(),
                ))
            }
        }
    }
}

fn synthetic_it(origin: uuid::Uuid) -> hir::Expression {
    hir::Expression::it_variable(Tag::unknown_span(origin), Tag::unknown_span(origin))
}
