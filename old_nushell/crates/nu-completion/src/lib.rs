pub(crate) mod command;
pub(crate) mod completer;
pub(crate) mod engine;
pub(crate) mod flag;
pub(crate) mod matchers;
pub(crate) mod path;
pub(crate) mod variable;

use nu_engine::EvaluationContext;
use nu_protocol::{SignatureRegistry, VariableRegistry};

use matchers::Matcher;

pub use completer::NuCompleter;

#[derive(Debug, Eq, PartialEq)]
pub struct Suggestion {
    pub display: String,
    pub replacement: String,
}

impl Suggestion {
    fn new(display: impl Into<String>, replacement: impl Into<String>) -> Self {
        Self {
            display: display.into(),
            replacement: replacement.into(),
        }
    }
}

pub trait CompletionContext {
    fn signature_registry(&self) -> &dyn SignatureRegistry;
    fn scope(&self) -> &dyn nu_parser::ParserScope;
    fn source(&self) -> &EvaluationContext;
    fn variable_registry(&self) -> &dyn VariableRegistry;
}

pub trait Completer<Context: CompletionContext> {
    fn complete(&self, ctx: &Context, partial: &str, matcher: &dyn Matcher) -> Vec<Suggestion>;
}
