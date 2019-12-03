use crate::commands::command::EvaluatedWholeStreamCommandArgs;
use crate::commands::cp::CopyArgs;
use crate::commands::mkdir::MkdirArgs;
use crate::commands::mv::MoveArgs;
use crate::commands::rm::RemoveArgs;
use crate::prelude::*;
use crate::stream::OutputStream;
use nu_errors::ShellError;
use nu_source::Tagged;
use std::path::PathBuf;

pub trait Shell: std::fmt::Debug {
    fn name(&self) -> String;
    fn homedir(&self) -> Option<PathBuf>;

    fn ls(
        &self,
        pattern: Option<Tagged<PathBuf>>,
        context: &RunnableContext,
        full: bool,
    ) -> Result<OutputStream, ShellError>;
    fn cd(&self, args: EvaluatedWholeStreamCommandArgs) -> Result<OutputStream, ShellError>;
    fn cp(&self, args: CopyArgs, name: Tag, path: &str) -> Result<OutputStream, ShellError>;
    fn mkdir(&self, args: MkdirArgs, name: Tag, path: &str) -> Result<OutputStream, ShellError>;
    fn mv(&self, args: MoveArgs, name: Tag, path: &str) -> Result<OutputStream, ShellError>;
    fn rm(&self, args: RemoveArgs, name: Tag, path: &str) -> Result<OutputStream, ShellError>;
    fn path(&self) -> String;
    fn pwd(&self, args: EvaluatedWholeStreamCommandArgs) -> Result<OutputStream, ShellError>;
    fn set_path(&mut self, path: String);

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> Result<(usize, Vec<rustyline::completion::Pair>), rustyline::error::ReadlineError>;

    fn hint(&self, _line: &str, _pos: usize, _ctx: &rustyline::Context<'_>) -> Option<String>;
}
