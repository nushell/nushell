use nu_errors::ShellError;

use crate::context;

#[derive(Debug, Eq, PartialEq)]
pub struct Suggestion {
    pub display: String,
    pub replacement: String,
}

pub struct Context<'a>(&'a context::Context, &'a rustyline::Context<'a>);

impl<'a> Context<'a> {
    pub fn new(a: &'a context::Context, b: &'a rustyline::Context<'a>) -> Context<'a> {
        Context(a, b)
    }
}

impl<'a> AsRef<context::Context> for Context<'a> {
    fn as_ref(&self) -> &context::Context {
        self.0
    }
}

impl<'a> AsRef<rustyline::Context<'a>> for Context<'a> {
    fn as_ref(&self) -> &rustyline::Context<'a> {
        self.1
    }
}

pub trait Completer {
    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> Result<(usize, Vec<Suggestion>), ShellError>;

    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<String>;
}
