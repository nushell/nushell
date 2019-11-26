pub mod classified;

use crate::commands::classified::ClassifiedCommand;
use crate::hir::expand_external_tokens::ExternalTokensShape;
use crate::hir::syntax_shape::{expand_syntax, ExpandContext};
use crate::hir::tokens_iterator::TokensIterator;
use nu_errors::ParseError;
use nu_source::{b, DebugDocBuilder, HasSpan, PrettyDebug, Span, Spanned, Tag, Tagged};

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

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExternalArg {
    pub arg: String,
    pub tag: Tag,
}

impl std::ops::Deref for ExternalArg {
    type Target = str;

    fn deref(&self) -> &str {
        &self.arg
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExternalArgs {
    pub list: Vec<ExternalArg>,
    pub span: Span,
}

impl ExternalArgs {
    pub fn iter(&self) -> impl Iterator<Item = &ExternalArg> {
        self.list.iter()
    }
}

impl std::ops::Deref for ExternalArgs {
    type Target = [ExternalArg];

    fn deref(&self) -> &[ExternalArg] {
        &self.list
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExternalCommand {
    pub name: String,

    pub name_tag: Tag,
    pub args: ExternalArgs,
}

impl PrettyDebug for ExternalCommand {
    fn pretty(&self) -> DebugDocBuilder {
        b::typed(
            "external command",
            b::description(&self.name)
                + b::preceded(
                    b::space(),
                    b::intersperse(
                        self.args.iter().map(|a| b::primitive(format!("{}", a.arg))),
                        b::space(),
                    ),
                ),
        )
    }
}

impl HasSpan for ExternalCommand {
    fn span(&self) -> Span {
        self.name_tag.span.until(self.args.span)
    }
}
