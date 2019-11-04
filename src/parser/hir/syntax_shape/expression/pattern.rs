use crate::parser::hir::syntax_shape::{
    expand_atom, expand_bare, expression::expand_file_path, AtomicToken, ExpandContext,
    ExpandExpression, ExpandSyntax, ExpansionRule, FallibleColorSyntax, FlatShape, ParseError,
};
use crate::parser::{hir, hir::TokensIterator, Operator, RawToken, TokenNode};
use crate::prelude::*;

#[derive(Debug, Copy, Clone)]
pub struct PatternShape;

#[cfg(not(coloring_in_tokens))]
impl FallibleColorSyntax for PatternShape {
    type Info = ();
    type Input = ();

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
        shapes: &mut Vec<Spanned<FlatShape>>,
    ) -> Result<(), ShellError> {
        token_nodes.atomic(|token_nodes| {
            let atom = expand_atom(token_nodes, "pattern", context, ExpansionRule::permissive())?;

            match &atom.item {
                AtomicToken::GlobPattern { .. } | AtomicToken::Word { .. } => {
                    shapes.push(FlatShape::GlobPattern.spanned(atom.span));
                    Ok(())
                }

                _ => Err(ShellError::type_error("pattern", atom.spanned_type_name())),
            }
        })
    }
}

#[cfg(coloring_in_tokens)]
impl FallibleColorSyntax for PatternShape {
    type Info = ();
    type Input = ();

    fn name(&self) -> &'static str {
        "PatternShape"
    }

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<(), ShellError> {
        token_nodes.atomic(|token_nodes| {
            let atom = expand_atom(token_nodes, "pattern", context, ExpansionRule::permissive())?;

            match &atom.item {
                AtomicToken::GlobPattern { .. } | AtomicToken::Word { .. } => {
                    token_nodes.color_shape(FlatShape::GlobPattern.spanned(atom.span));
                    Ok(())
                }

                other => Err(ShellError::type_error(
                    "pattern",
                    other.type_name().spanned(atom.span),
                )),
            }
        })
    }
}

impl ExpandExpression for PatternShape {
    fn name(&self) -> &'static str {
        "glob pattern"
    }

    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
        context: &ExpandContext,
    ) -> Result<hir::Expression, ParseError> {
        let atom = expand_atom(token_nodes, "pattern", context, ExpansionRule::new())?;

        match atom.item {
            AtomicToken::Word { text: body }
            | AtomicToken::String { body }
            | AtomicToken::GlobPattern { pattern: body } => {
                let path = expand_file_path(body.slice(context.source), context);
                return Ok(hir::Expression::pattern(path.to_string_lossy(), atom.span));
            }
            _ => return atom.into_hir(context, "pattern"),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct BarePatternShape;

impl ExpandSyntax for BarePatternShape {
    type Output = Span;

    fn name(&self) -> &'static str {
        "bare pattern"
    }

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Span, ParseError> {
        expand_bare(token_nodes, context, |token| match token {
            TokenNode::Token(Spanned {
                item: RawToken::Bare,
                ..
            })
            | TokenNode::Token(Spanned {
                item: RawToken::Operator(Operator::Dot),
                ..
            })
            | TokenNode::Token(Spanned {
                item: RawToken::GlobPattern,
                ..
            }) => true,

            _ => false,
        })
    }
}
