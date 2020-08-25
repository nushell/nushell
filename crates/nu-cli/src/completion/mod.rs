pub(crate) mod command;
pub(crate) mod engine;
pub(crate) mod flag;
pub(crate) mod path;

pub(crate) mod matchers;

use nu_errors::ShellError;

use crate::context;

use crate::completion::matchers::Matcher;

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
    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
        matcher: &Box<dyn Matcher>,
    ) -> Result<(usize, Vec<Suggestion>), ShellError>;
}
