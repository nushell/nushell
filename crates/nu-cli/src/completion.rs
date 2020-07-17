use nu_errors::ShellError;

#[derive(Debug, Eq, PartialEq)]
pub struct Suggestion {
    pub display: String,
    pub replacement: String,
}

pub struct Context<'a>(pub &'a rustyline::Context<'a>);

impl<'a> AsRef<rustyline::Context<'a>> for Context<'a> {
    fn as_ref(&self) -> &rustyline::Context<'a> {
        self.0
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
