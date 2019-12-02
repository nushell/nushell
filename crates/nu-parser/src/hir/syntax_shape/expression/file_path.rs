use crate::hir::syntax_shape::expression::atom::{
    expand_atom, ExpansionRule, UnspannedAtomicToken,
};
use crate::hir::syntax_shape::{
    expression::expand_file_path, ExpandContext, ExpandExpression, FallibleColorSyntax, FlatShape,
};
use crate::{hir, hir::TokensIterator};
use nu_errors::{ParseError, ShellError};
use nu_source::SpannedItem;

#[derive(Debug, Copy, Clone)]
pub struct FilePathShape;

#[cfg(not(coloring_in_tokens))]
impl FallibleColorSyntax for FilePathShape {
    type Info = ();
    type Input = ();

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
        shapes: &mut Vec<nu_source::Spanned<FlatShape>>,
    ) -> Result<(), ShellError> {
        let atom = expand_atom(
            token_nodes,
            "file path",
            context,
            ExpansionRule::permissive(),
        );

        let atom = match atom {
            Err(_) => return Ok(()),
            Ok(atom) => atom,
        };

        match &atom.unspanned {
            UnspannedAtomicToken::Word { .. }
            | UnspannedAtomicToken::String { .. }
            | UnspannedAtomicToken::Number { .. }
            | UnspannedAtomicToken::Size { .. } => {
                shapes.push(FlatShape::Path.spanned(atom.span));
            }

            _ => atom.color_tokens(shapes),
        }

        Ok(())
    }
}

#[cfg(coloring_in_tokens)]
impl FallibleColorSyntax for FilePathShape {
    type Info = ();
    type Input = ();

    fn name(&self) -> &'static str {
        "FilePathShape"
    }

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<(), ShellError> {
        let atom = expand_atom(
            token_nodes,
            "file path",
            context,
            ExpansionRule::permissive(),
        );

        let atom = match atom {
            Err(_) => return Ok(()),
            Ok(atom) => atom,
        };

        match atom.unspanned {
            UnspannedAtomicToken::Word { .. }
            | UnspannedAtomicToken::String { .. }
            | UnspannedAtomicToken::Number { .. }
            | UnspannedAtomicToken::Size { .. } => {
                token_nodes.color_shape(FlatShape::Path.spanned(atom.span));
            }

            _ => token_nodes.mutate_shapes(|shapes| atom.color_tokens(shapes)),
        }

        Ok(())
    }
}

impl ExpandExpression for FilePathShape {
    fn name(&self) -> &'static str {
        "file path"
    }

    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
        context: &ExpandContext,
    ) -> Result<hir::Expression, ParseError> {
        let atom = expand_atom(token_nodes, "file path", context, ExpansionRule::new())?;

        match atom.unspanned {
            UnspannedAtomicToken::Word { text: body } | UnspannedAtomicToken::String { body } => {
                let path = expand_file_path(body.slice(context.source), context);
                return Ok(hir::Expression::file_path(path, atom.span));
            }

            UnspannedAtomicToken::Number { .. } | UnspannedAtomicToken::Size { .. } => {
                let path = atom.span.slice(context.source);
                return Ok(hir::Expression::file_path(path, atom.span));
            }

            _ => return atom.into_hir(context, "file path"),
        }
    }
}
