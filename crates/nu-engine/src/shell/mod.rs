use nu_stream::OutputStream;

use crate::command_args::EvaluatedWholeStreamCommandArgs;
use crate::maybe_text_codec::StringOrBinary;
pub use crate::shell::shell_args::{CdArgs, CopyArgs, LsArgs, MkdirArgs, MvArgs, RemoveArgs};
use encoding_rs::Encoding;
use futures::stream::BoxStream;
use nu_errors::ShellError;
use nu_source::{Span, Tag};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub(crate) mod help_shell;
pub(crate) mod painter;
pub(crate) mod palette;
pub(crate) mod shell_args;
pub(crate) mod shell_manager;
pub(crate) mod value_shell;

pub trait Shell: std::fmt::Debug {
    fn name(&self) -> String;
    fn homedir(&self) -> Option<PathBuf>;

    fn ls(
        &self,
        args: LsArgs,
        name: Tag,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<OutputStream, ShellError>;
    fn cd(&self, args: CdArgs, name: Tag) -> Result<OutputStream, ShellError>;
    fn cp(&self, args: CopyArgs, name: Tag, path: &str) -> Result<OutputStream, ShellError>;
    fn mkdir(&self, args: MkdirArgs, name: Tag, path: &str) -> Result<OutputStream, ShellError>;
    fn mv(&self, args: MvArgs, name: Tag, path: &str) -> Result<OutputStream, ShellError>;
    fn rm(&self, args: RemoveArgs, name: Tag, path: &str) -> Result<OutputStream, ShellError>;
    fn path(&self) -> String;
    fn pwd(&self, args: EvaluatedWholeStreamCommandArgs) -> Result<OutputStream, ShellError>;
    fn set_path(&mut self, path: String);
    fn open(
        &self,
        path: &Path,
        name: Span,
        with_encoding: Option<&'static Encoding>,
    ) -> Result<BoxStream<'static, Result<StringOrBinary, ShellError>>, ShellError>;
    fn save(
        &mut self,
        path: &Path,
        contents: &[u8],
        name: Span,
    ) -> Result<OutputStream, ShellError>;
}
