pub mod classified;

use crate::commands::classified::external::{ExternalArg, ExternalArgs, ExternalCommand};
use crate::commands::classified::ClassifiedCommand;
use crate::hir::expand_external_tokens::ExternalTokensShape;
use crate::hir::tokens_iterator::TokensIterator;
use nu_errors::ParseError;
use nu_source::{Spanned, Tagged};

// Classify this command as an external command, which doesn't give special meaning
// to nu syntactic constructs, and passes all arguments to the external command as
// strings.
pub(crate) fn external_command(
    tokens: &mut TokensIterator,
    name: Tagged<&str>,
) -> Result<ClassifiedCommand, ParseError> {
    let Spanned { item, span } = tokens.expand_syntax(ExternalTokensShape)?.tokens;

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
