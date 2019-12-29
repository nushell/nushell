use crate::hir::syntax_shape::{
    expand_atom, expand_bare, expression::expand_file_path, ExpandContext, ExpandExpression,
    ExpandSyntax, ExpansionRule, FallibleColorSyntax, FlatShape, UnspannedAtomicToken,
};
use crate::parse::operator::EvaluationOperator;
use crate::parse::tokens::{Token, UnspannedToken};
use crate::{hir, hir::TokensIterator, TokenNode};
use nu_errors::{ParseError, ShellError};

use nu_protocol::ShellTypeName;
use nu_source::{Span, SpannedItem};

#[derive(Debug, Copy, Clone)]
pub struct PatternShape;

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

            match &atom.unspanned {
                UnspannedAtomicToken::GlobPattern { .. } | UnspannedAtomicToken::Word { .. } => {
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
        let atom = expand_atom(
            token_nodes,
            "pattern",
            context,
            ExpansionRule::new().allow_external_word(),
        )?;

        match atom.unspanned {
            UnspannedAtomicToken::Word { text: body }
            | UnspannedAtomicToken::String { body }
            | UnspannedAtomicToken::ExternalWord { text: body }
            | UnspannedAtomicToken::GlobPattern { pattern: body } => {
                let path = expand_file_path(body.slice(context.source), context);
                Ok(hir::Expression::pattern(path.to_string_lossy(), atom.span))
            }
            _ => atom.to_hir(context, "pattern"),
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
            TokenNode::Token(Token {
                unspanned: UnspannedToken::Bare,
                ..
            })
            | TokenNode::Token(Token {
                unspanned: UnspannedToken::EvaluationOperator(EvaluationOperator::Dot),
                ..
            })
            | TokenNode::Token(Token {
                unspanned: UnspannedToken::GlobPattern,
                ..
            }) => true,

            _ => false,
        })
    }
}
