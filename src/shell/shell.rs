use crate::commands::command::CallInfo;
use crate::errors::ShellError;
use crate::stream::{InputStream, OutputStream};
use rustyline::{completion::Completer, hint::Hinter};

pub trait Shell
where
    Self: Completer<Candidate = rustyline::completion::Pair>,
    Self: Hinter,
{
    fn ls(&self, call_info: CallInfo, input: InputStream) -> Result<OutputStream, ShellError>;
    fn cd(&self, call_info: CallInfo, input: InputStream) -> Result<OutputStream, ShellError>;
    fn path(&self) -> std::path::PathBuf;
    fn set_path(&mut self, path: &std::path::PathBuf);
}
