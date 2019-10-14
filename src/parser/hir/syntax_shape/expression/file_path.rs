use crate::parser::hir::syntax_shape::expression::atom::{expand_atom, AtomicToken, ExpansionRule};
use crate::parser::hir::syntax_shape::{
    expression::expand_file_path, ExpandContext, ExpandExpression, FallibleColorSyntax, FlatShape,
};
use crate::parser::{hir, hir::TokensIterator};
use crate::prelude::*;

#[derive(Debug, Copy, Clone)]
pub struct FilePathShape;

impl FallibleColorSyntax for FilePathShape {
    type Info = ();
    type Input = ();

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
        shapes: &mut Vec<Spanned<FlatShape>>,
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

        match atom.item {
            AtomicToken::Word { .. }
            | AtomicToken::String { .. }
            | AtomicToken::Number { .. }
            | AtomicToken::Size { .. } => {
                shapes.push(FlatShape::Path.spanned(atom.span));
            }

            _ => atom.color_tokens(shapes),
        }

        Ok(())
    }
}

impl ExpandExpression for FilePathShape {
    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
        context: &ExpandContext,
    ) -> Result<hir::Expression, ShellError> {
        let atom = expand_atom(token_nodes, "file path", context, ExpansionRule::new())?;

        match atom.item {
            AtomicToken::Word { text: body } | AtomicToken::String { body } => {
                let path = expand_file_path(body.slice(context.source), context);
                return Ok(hir::Expression::file_path(path, atom.span));
            }

            AtomicToken::Number { .. } | AtomicToken::Size { .. } => {
                let path = atom.span.slice(context.source);
                return Ok(hir::Expression::file_path(path, atom.span));
            }

            _ => return atom.into_hir(context, "file path"),
        }
    }
}
