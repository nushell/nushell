use crate::parser::hir::syntax_shape::{
    expand_atom, expand_bare, expand_syntax, expression::expand_file_path, parse_single_node,
    AtomicToken, ExpandContext, ExpandExpression, ExpandSyntax, ExpansionRule, FallibleColorSyntax,
    FlatShape,
};
use crate::parser::{hir, hir::TokensIterator, Operator, RawToken, TokenNode};
use crate::prelude::*;

#[derive(Debug, Copy, Clone)]
pub struct PatternShape;

impl FallibleColorSyntax for PatternShape {
    type Info = ();
    type Input = ();

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
        shapes: &mut Vec<Tagged<FlatShape>>,
    ) -> Result<(), ShellError> {
        token_nodes.atomic(|token_nodes| {
            let atom = expand_atom(token_nodes, "pattern", context, ExpansionRule::permissive())?;

            match &atom.item {
                AtomicToken::GlobPattern { .. } | AtomicToken::Word { .. } => {
                    shapes.push(FlatShape::GlobPattern.tagged(atom.tag));
                    Ok(())
                }

                _ => Err(ShellError::type_error("pattern", atom.tagged_type_name())),
            }
        })
    }
}

impl ExpandExpression for PatternShape {
    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
        context: &ExpandContext,
    ) -> Result<hir::Expression, ShellError> {
        let pattern = expand_syntax(&BarePatternShape, token_nodes, context);

        match pattern {
            Ok(tag) => {
                return Ok(hir::Expression::pattern(tag));
            }
            Err(_) => {}
        }

        parse_single_node(token_nodes, "Pattern", |token, token_tag, _| {
            Ok(match token {
                RawToken::GlobPattern => {
                    return Err(ShellError::unreachable(
                        "glob pattern after glob already returned",
                    ))
                }
                RawToken::Operator(..) => {
                    return Err(ShellError::unreachable("dot after glob already returned"))
                }
                RawToken::Bare => {
                    return Err(ShellError::unreachable("bare after glob already returned"))
                }

                RawToken::Variable(tag) if tag.slice(context.source) == "it" => {
                    hir::Expression::it_variable(tag, token_tag)
                }
                RawToken::Variable(tag) => hir::Expression::variable(tag, token_tag),
                RawToken::ExternalCommand(tag) => hir::Expression::external_command(tag, token_tag),
                RawToken::ExternalWord => return Err(ShellError::invalid_external_word(token_tag)),
                RawToken::Number(_) => hir::Expression::bare(token_tag),

                RawToken::String(tag) => hir::Expression::file_path(
                    expand_file_path(tag.slice(context.source), context),
                    token_tag,
                ),
            })
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub struct BarePatternShape;

impl ExpandSyntax for BarePatternShape {
    type Output = Tag;

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Tag, ShellError> {
        expand_bare(token_nodes, context, |token| match token {
            TokenNode::Token(Tagged {
                item: RawToken::Bare,
                ..
            })
            | TokenNode::Token(Tagged {
                item: RawToken::Operator(Operator::Dot),
                ..
            })
            | TokenNode::Token(Tagged {
                item: RawToken::GlobPattern,
                ..
            }) => true,

            _ => false,
        })
    }
}
