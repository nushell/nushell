pub mod classified;

use crate::commands::classified::external::{ExternalArg, ExternalArgs, ExternalCommand};
use crate::commands::classified::ClassifiedCommand;
use crate::hir::expand_external_tokens::ExternalTokensShape;
use crate::hir::syntax_shape::{expand_syntax, ExpandContext};
use crate::hir::tokens_iterator::TokensIterator;
use nu_errors::ParseError;
use nu_source::{Spanned, Tagged};

// Classify this command as an external command, which doesn't give special meaning
// to nu syntactic constructs, and passes all arguments to the external command as
// strings.
pub(crate) fn external_command(
    tokens: &mut TokensIterator,
    context: &ExpandContext,
    name: Tagged<&str>,
) -> Result<ClassifiedCommand, ParseError> {
    let Spanned { item, span } = expand_syntax(&ExternalTokensShape, tokens, context)?.tokens;

    Ok(ClassifiedCommand::External(ExternalCommand {
        name: name.to_string(),
        name_tag: name.tag(),
        args: ExternalArgs {
            list: item
                .iter()
                .map(|x| ExternalArg {
                    tag: x.span.into(),
                    arg: x.item.clone(),
                })
                .collect(),
            span,
        },
    }))
}
