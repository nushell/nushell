pub(crate) mod command;
pub(crate) mod engine;
pub(crate) mod flag;
pub(crate) mod matchers;
pub(crate) mod path;

use crate::context;
use matchers::Matcher;

#[derive(Debug, Eq, PartialEq)]
pub struct Suggestion {
    pub display: String,
    pub replacement: String,
}

pub struct Context<'a>(&'a context::Context);

impl<'a> Context<'a> {
    pub fn new(a: &'a context::Context) -> Context<'a> {
        Context(a)
    }
}

impl<'a> AsRef<context::Context> for Context<'a> {
    fn as_ref(&self) -> &context::Context {
        self.0
    }
}

pub trait Completer {
    fn complete(&self, ctx: &Context<'_>, partial: &str, matcher: &dyn Matcher) -> Vec<Suggestion>;
}
