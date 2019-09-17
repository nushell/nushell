use crate::parser::hir::syntax_shape::{
    expand_syntax, expression::expand_file_path, parse_single_node, BarePathShape, ExpandContext,
    ExpandExpression,
};
use crate::parser::{hir, hir::TokensIterator, RawToken};
use crate::prelude::*;

#[derive(Debug, Copy, Clone)]
pub struct FilePathShape;

impl ExpandExpression for FilePathShape {
    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
        context: &ExpandContext,
    ) -> Result<hir::Expression, ShellError> {
        let bare = expand_syntax(&BarePathShape, token_nodes, context);

        match bare {
            Ok(tag) => {
                let string = tag.slice(context.source);
                let path = expand_file_path(string, context);
                return Ok(hir::Expression::file_path(path, tag));
            }
            Err(_) => {}
        }

        parse_single_node(token_nodes, "Path", |token, token_tag| {
            Ok(match token {
                RawToken::GlobPattern => {
                    return Err(ShellError::type_error(
                        "Path",
                        "glob pattern".tagged(token_tag),
                    ))
                }
                RawToken::Operator(..) => {
                    return Err(ShellError::type_error("Path", "operator".tagged(token_tag)))
                }
                RawToken::Variable(tag) if tag.slice(context.source) == "it" => {
                    hir::Expression::it_variable(tag, token_tag)
                }
                RawToken::Variable(tag) => hir::Expression::variable(tag, token_tag),
                RawToken::ExternalCommand(tag) => hir::Expression::external_command(tag, token_tag),
                RawToken::ExternalWord => return Err(ShellError::invalid_external_word(token_tag)),
                RawToken::Number(_) => hir::Expression::bare(token_tag),
                RawToken::Size(_, _) => hir::Expression::bare(token_tag),
                RawToken::Bare => hir::Expression::file_path(
                    expand_file_path(token_tag.slice(context.source), context),
                    token_tag,
                ),

                RawToken::String(tag) => hir::Expression::file_path(
                    expand_file_path(tag.slice(context.source), context),
                    token_tag,
                ),
            })
        })
    }
}
