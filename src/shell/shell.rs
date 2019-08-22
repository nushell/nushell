use crate::commands::command::EvaluatedWholeStreamCommandArgs;
use crate::commands::cp::CopyArgs;
use crate::commands::mkdir::MkdirArgs;
use crate::commands::mv::MoveArgs;
use crate::commands::rm::RemoveArgs;
use crate::context::SourceMap;
use crate::errors::ShellError;
use crate::prelude::*;
use crate::stream::OutputStream;

pub trait Shell: std::fmt::Debug {
    fn name(&self, source_map: &SourceMap) -> String;
    fn ls(&self, args: EvaluatedWholeStreamCommandArgs) -> Result<OutputStream, ShellError>;
    fn cd(&self, args: EvaluatedWholeStreamCommandArgs) -> Result<OutputStream, ShellError>;
    fn cp(
        &self,
        args: CopyArgs,
        name: Span,
        path: &str,
    ) -> Result<VecDeque<ReturnValue>, ShellError>;
    fn mkdir(
        &self,
        args: MkdirArgs,
        name: Span,
        path: &str,
    ) -> Result<VecDeque<ReturnValue>, ShellError>;
    fn mv(
        &self,
        args: MoveArgs,
        name: Span,
        path: &str,
    ) -> Result<VecDeque<ReturnValue>, ShellError>;
    fn rm(
        &self,
        args: RemoveArgs,
        name: Span,
        path: &str,
    ) -> Result<VecDeque<ReturnValue>, ShellError>;
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
