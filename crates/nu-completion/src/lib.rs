pub(crate) mod command;
pub(crate) mod completer;
pub(crate) mod engine;
pub(crate) mod flag;
pub(crate) mod matchers;
pub(crate) mod path;

use matchers::Matcher;

pub use completer::NuCompleter;

#[derive(Debug, Eq, PartialEq)]
pub struct Suggestion {
    pub display: String,
    pub replacement: String,
}

pub trait CompletionContext {
    fn signature_registry(&self) -> &dyn nu_parser::ParserScope;
}

pub trait Completer<Context: CompletionContext> {
    fn complete(&self, ctx: &Context, partial: &str, matcher: &dyn Matcher) -> Vec<Suggestion>;
}
