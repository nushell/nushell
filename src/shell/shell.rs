use crate::commands::command::EvaluatedStaticCommandArgs;
use crate::context::SourceMap;
use crate::errors::ShellError;
use crate::stream::OutputStream;

pub trait Shell {
    fn name(&self, source_map: &SourceMap) -> String;
    fn ls(&self, args: EvaluatedStaticCommandArgs) -> Result<OutputStream, ShellError>;
    fn cd(&self, args: EvaluatedStaticCommandArgs) -> Result<OutputStream, ShellError>;
    fn path(&self) -> String;
    fn set_path(&mut self, path: String);

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> Result<(usize, Vec<rustyline::completion::Pair>), rustyline::error::ReadlineError>;

    fn hint(&self, _line: &str, _pos: usize, _ctx: &rustyline::Context<'_>) -> Option<String>;
}
