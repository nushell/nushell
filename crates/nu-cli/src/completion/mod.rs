pub(crate) mod command;
pub(crate) mod engine;
pub(crate) mod flag;
pub(crate) mod matchers;
pub(crate) mod path;

use crate::evaluation_context::EvaluationContext;
use matchers::Matcher;

#[derive(Debug, Eq, PartialEq)]
pub struct Suggestion {
    pub display: String,
    pub replacement: String,
}

pub struct CompletionContext<'a>(&'a EvaluationContext);

impl<'a> CompletionContext<'a> {
    pub fn new(a: &'a EvaluationContext) -> CompletionContext<'a> {
        CompletionContext(a)
    }
}

impl<'a> AsRef<EvaluationContext> for CompletionContext<'a> {
    fn as_ref(&self) -> &EvaluationContext {
        self.0
    }
}

pub trait Completer {
    fn complete(
        &self,
        ctx: &CompletionContext<'_>,
        partial: &str,
        matcher: &dyn Matcher,
    ) -> Vec<Suggestion>;
}
