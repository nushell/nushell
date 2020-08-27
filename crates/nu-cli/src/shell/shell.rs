use crate::commands::cd::CdArgs;
use crate::commands::classified::maybe_text_codec::StringOrBinary;
use crate::commands::command::EvaluatedWholeStreamCommandArgs;
use crate::commands::cp::CopyArgs;
use crate::commands::ls::LsArgs;
use crate::commands::mkdir::MkdirArgs;
use crate::commands::move_::mv::Arguments as MvArgs;
use crate::commands::rm::RemoveArgs;
use crate::prelude::*;
use crate::stream::OutputStream;

use encoding_rs::Encoding;
use nu_errors::ShellError;
use std::path::PathBuf;

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
        path: &PathBuf,
        name: Span,
        with_encoding: Option<&'static Encoding>,
    ) -> Result<BoxStream<'static, Result<StringOrBinary, ShellError>>, ShellError>;
    fn save(
        &mut self,
        path: &PathBuf,
        contents: &[u8],
        name: Span,
    ) -> Result<OutputStream, ShellError>;
}
