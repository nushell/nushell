use crate::commands::command::{CallInfo, EvaluatedStaticCommandArgs};
use crate::errors::ShellError;
use crate::stream::{InputStream, OutputStream};

pub trait Shell {
    fn name(&self) -> String;
    fn ls(&self, args: EvaluatedStaticCommandArgs) -> Result<OutputStream, ShellError>;
    fn cd(&self, call_info: CallInfo, input: InputStream) -> Result<OutputStream, ShellError>;
    fn path(&self) -> String;
    fn set_path(&mut self, path: String);

    // fn complete(
    //     &self,
    //     line: &str,
    //     pos: usize,
    //     ctx: &rustyline::Context<'_>,
    // ) -> Result<(usize, Vec<CompletionPair>), ReadlineError>;

    fn hint(&self, _line: &str, _pos: usize, _ctx: &rustyline::Context<'_>) -> Option<String>;
}
